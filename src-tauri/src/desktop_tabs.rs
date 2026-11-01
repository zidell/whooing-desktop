//! Desktop tab chrome lives in a trusted local webview. Content webviews never
//! receive tab-management IPC permissions, including when showing external sites.
use std::{
  path::Path,
  sync::{mpsc, Mutex},
};

use serde::Serialize;
use tauri::{
  menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder},
  webview::{NewWindowResponse, WebviewBuilder},
  AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WebviewWindowBuilder,
  WindowEvent,
};

use super::{
  desktop_info_script, is_app_origin, is_internal_url, link_action, must_open_in_browser,
  open_in_browser, LinkAction, LOADING_SCRIPT,
};

const CHROME_HEIGHT: f64 = 32.;

#[derive(Clone, Serialize)]
pub struct Tab {
  id: String,
  title: String,
}

#[derive(Clone, Default, Serialize)]
pub struct Snapshot {
  tabs: Vec<Tab>,
  active: String,
  tab_bar_visible: bool,
  tab_bar_height: f64,
  can_reopen: bool,
}

#[derive(Clone)]
struct ClosedTab {
  url: Url,
  index: usize,
}

pub struct DesktopTabs {
  home: Url,
  snapshot: Mutex<Snapshot>,
  sender: mpsc::Sender<Action>,
  closed: Mutex<Vec<ClosedTab>>,
  context_menu: Mutex<Option<Menu<tauri::Wry>>>,
  zoom_percent: Mutex<i32>,
}

enum Action {
  Open(Url),
  OpenAfter(Url, String),
  Select(String),
  Close(String),
  Cycle(i32),
  Reload(String),
  Zoom(i32),
  Resize,
  DeepLink(Url),
  SelectDigit(usize),
  Duplicate(String),
  Move { id: String, before: Option<String> },
  MoveBy(String, i32),
  Reopen,
  CloseOthers(String),
  CloseRight(String),
  History(String, bool),
  Context(String, f64, f64),
}

// One native app preference, independent of website storage and tab lifetimes.
fn load_zoom(path: &Path) -> i32 {
  match std::fs::read(path) {
    Ok(bytes) => serde_json::from_slice::<i32>(&bytes)
      .ok()
      .filter(|percent| (50..=300).contains(percent))
      .unwrap_or(100),
    Err(error) => {
      if error.kind() != std::io::ErrorKind::NotFound {
        log::warn!("Could not load zoom preference: {error}");
      }
      100
    }
  }
}

fn save_zoom(path: &Path, percent: i32) -> std::io::Result<()> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(path, percent.to_string())
}

fn next_zoom(percent: i32, delta: i32) -> i32 {
  if delta == 0 {
    100
  } else {
    (percent + delta).clamp(50, 300)
  }
}

fn send(app: &AppHandle, action: Action) {
  if let Some(state) = app.try_state::<DesktopTabs>() {
    let _ = state.sender.send(action);
  }
}

pub fn open(app: &AppHandle, url: Url) {
  send(app, Action::Open(url));
}
pub fn deep_link(app: &AppHandle, url: Url) {
  send(app, Action::DeepLink(url));
}

fn snapshot(app: &AppHandle) -> Snapshot {
  let mut state = app.state::<DesktopTabs>().snapshot.lock().unwrap().clone();
  state.tab_bar_visible = super::tab_bar_visible(state.tabs.len());
  state.tab_bar_height = CHROME_HEIGHT;
  state.can_reopen = !app.state::<DesktopTabs>().closed.lock().unwrap().is_empty();
  state
}

fn publish(app: &AppHandle) {
  let state = snapshot(app);
  // Target the local shell, never broadcast URLs (including OAuth query strings)
  // to remote content webviews.
  let _ = app.emit_to(tauri::EventTarget::webview("main"), "tabs-changed", &state);
  if let Some(window) = app.get_window("main") {
    let title = state
      .tabs
      .iter()
      .find(|tab| tab.id == state.active)
      .map(|tab| format!("{} — Whooing", tab.title))
      .unwrap_or_else(|| "Whooing".into());
    let _ = window.set_title(&title);
  }
}

fn require_shell(webview: &tauri::Webview) -> Result<(), String> {
  if webview.label() == "main" && webview.url().is_ok_and(|url| is_internal_url(&url)) {
    Ok(())
  } else {
    Err("Tab controls are only available to the local shell".into())
  }
}

#[tauri::command]
pub fn tabs_snapshot(app: AppHandle, webview: tauri::Webview) -> Result<Snapshot, String> {
  require_shell(&webview)?;
  Ok(snapshot(&app))
}

#[tauri::command]
pub fn tabs_action(
  app: AppHandle,
  webview: tauri::Webview,
  action: String,
  id: Option<String>,
  url: Option<String>,
  before_id: Option<String>,
  x: Option<f64>,
  y: Option<f64>,
) -> Result<(), String> {
  require_shell(&webview)?;
  let id = id.unwrap_or_else(|| snapshot(&app).active);
  let action = match action.as_str() {
    "open" => {
      let url: Url = match url {
        Some(url) => url.parse().map_err(|_| "Invalid URL")?,
        None => app.state::<DesktopTabs>().home.clone(),
      };
      match link_action(&url, true, false) {
        LinkAction::Tab => Action::Open(url),
        LinkAction::Browser => {
          open_in_browser(&app, &url);
          return Ok(());
        }
        _ => return Err("Only web links can be opened in tabs".into()),
      }
    }
    "select" => Action::Select(id),
    "close" => Action::Close(id),
    "next" => Action::Cycle(1),
    "previous" => Action::Cycle(-1),
    "reload" => Action::Reload(id),
    "zoom-in" => Action::Zoom(10),
    "zoom-out" => Action::Zoom(-10),
    "zoom-reset" => Action::Zoom(0),
    "duplicate" => Action::Duplicate(id),
    "move" => Action::Move {
      id,
      before: before_id,
    },
    "move-left" => Action::MoveBy(id, -1),
    "move-right" => Action::MoveBy(id, 1),
    "reopen" => Action::Reopen,
    "close-others" => Action::CloseOthers(id),
    "close-right" => Action::CloseRight(id),
    "back" => Action::History(id, false),
    "forward" => Action::History(id, true),
    "context" => Action::Context(
      id,
      x.filter(|x| x.is_finite()).unwrap_or(0.),
      y.filter(|y| y.is_finite()).unwrap_or(CHROME_HEIGHT),
    ),
    name if name.starts_with("select-") => {
      let digit = name[7..]
        .parse::<usize>()
        .map_err(|_| "Invalid tab number")?;
      if !(1..=9).contains(&digit) {
        return Err("Invalid tab number".into());
      }
      Action::SelectDigit(digit)
    }
    _ => return Err("Unknown tab action".into()),
  };
  app
    .state::<DesktopTabs>()
    .sender
    .send(action)
    .map_err(|_| "Tab worker stopped".into())
}

// GTK handles Ctrl+Tab as focus traversal before WebKit's DOM keydown path.
// Handle both directions at the host window, including XKB's shifted Tab keysym.
#[cfg(target_os = "linux")]
fn linux_tab_cycle(key: gtk::gdk::keys::Key, modifiers: gtk::gdk::ModifierType) -> Option<i32> {
  use gtk::gdk::{keys::constants, ModifierType};
  if !modifiers.contains(ModifierType::CONTROL_MASK)
    || modifiers.intersects(ModifierType::MOD1_MASK | ModifierType::SUPER_MASK | ModifierType::META_MASK)
    || !matches!(key, constants::Tab | constants::ISO_Left_Tab | constants::KP_Tab)
  {
    return None;
  }
  Some(if modifiers.contains(ModifierType::SHIFT_MASK) || key == constants::ISO_Left_Tab {
    -1
  } else {
    1
  })
}

pub fn setup(app: &tauri::App, start_url: Url, home: Url) -> tauri::Result<()> {
  let (sender, receiver) = mpsc::channel();
  app.manage(DesktopTabs {
    home,
    snapshot: Mutex::default(),
    sender,
    closed: Mutex::default(),
    context_menu: Mutex::default(),
    zoom_percent: Mutex::new(load_zoom(&app.path().app_config_dir()?.join("zoom.json"))),
  });
  app.on_menu_event(on_menu_event);
  #[cfg(target_os = "macos")]
  install_macos_menu(app)?;
  let shell = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("tabs.html".into()))
    .background_color(tauri::webview::Color(37, 37, 37, 255))
    .title("Whooing")
    .inner_size(1280., 800.)
    .min_inner_size(380., 600.)
    .disable_drag_drop_handler()
    .on_navigation(is_internal_url)
    .on_new_window(|_, _| NewWindowResponse::Deny)
    .build()?;
  #[cfg(target_os = "linux")]
  {
    use gtk::prelude::*;
    let handle = app.handle().clone();
    shell.gtk_window()?.connect_key_press_event(move |_, event| {
      if let Some(direction) = linux_tab_cycle(event.keyval(), event.state()) {
        send(&handle, Action::Cycle(direction));
        gtk::glib::Propagation::Stop
      } else {
        gtk::glib::Propagation::Proceed
      }
    });
  }
  let handle = app.handle().clone();
  shell.on_window_event(move |event| {
    if matches!(
      event,
      WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
    ) {
      send(&handle, Action::Resize);
    }
  });
  let handle = app.handle().clone();
  // add_child must not run inside a synchronous WebView2 callback. All tab
  // mutations are serialized on this worker; locks are never held while calling
  // WebView2 (title/navigation callbacks can run during creation).
  std::thread::spawn(move || {
    let mut next_id = 0;
    while let Ok(action) = receiver.recv() {
      if let Err(error) = perform(&handle, action, &mut next_id) {
        log::warn!("Tab operation failed: {error}");
        let _ = handle.emit_to(
          tauri::EventTarget::webview("main"),
          "tabs-error",
          "탭을 열거나 변경하지 못했습니다. 다시 시도해 주세요.",
        );
      }
    }
  });
  open(app.handle(), start_url);
  Ok(())
}

fn select(app: &AppHandle, id: &str) -> tauri::Result<()> {
  let current = snapshot(app);
  if !current.tabs.iter().any(|tab| tab.id == id) {
    return Ok(());
  }
  for tab in &current.tabs {
    if let Some(view) = app.get_webview(&tab.id) {
      if tab.id == id {
        view.show()?;
        view.set_focus()?;
      } else {
        view.hide()?;
      }
    }
  }
  app.state::<DesktopTabs>().snapshot.lock().unwrap().active = id.into();
  publish(app);
  Ok(())
}

fn bounds(app: &AppHandle) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
  #[cfg(not(target_os = "macos"))]
  let (top_inset, size) = {
    let window = app.get_window("main").ok_or(tauri::Error::WindowNotFound)?;
    let size = window
      .inner_size()?
      .to_logical::<f64>(window.scale_factor()?);
    (0., size)
  };
  #[cfg(target_os = "macos")]
  let (top_inset, size) = {
    // macOS child views extend beneath the title bar. The local shell's HTML
    // respects that safe area, so content must reserve it as well as the tabs.
    let shell = app.get_webview("main").ok_or(tauri::Error::WindowNotFound)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    shell.with_webview(move |platform| unsafe {
      let view = &*platform.inner().cast::<objc2_web_kit::WKWebView>();
      let parent = view.superview().unwrap();
      let size = parent.bounds().size;
      let top_inset = parent.safeAreaInsets().top;
      let _ = sender.send((
        top_inset,
        LogicalSize::new(size.width, size.height - top_inset),
      ));
    })?;
    receiver.recv().map_err(std::io::Error::other)?
  };
  let chrome_height = if snapshot(app).tab_bar_visible {
    CHROME_HEIGHT
  } else {
    0.
  };
  Ok((
    LogicalPosition::new(0., top_inset + chrome_height),
    LogicalSize::new(size.width, (size.height - chrome_height).max(1.)),
  ))
}

#[cfg(not(target_os = "linux"))]
fn resize(app: &AppHandle) -> tauri::Result<()> {
  let (position, size) = bounds(app)?;
  for tab in snapshot(app).tabs {
    if let Some(view) = app.get_webview(&tab.id) {
      view.set_bounds(tauri::Rect {
        position: position.into(),
        size: size.into(),
      })?;
    }
  }
  Ok(())
}

#[cfg(target_os = "linux")]
fn resize(app: &AppHandle) -> tauri::Result<()> {
  use gtk::prelude::*;
  let visible = snapshot(app).tab_bar_visible;
  let window = app.get_window("main").ok_or(tauri::Error::WindowNotFound)?;
  for view in window.webviews() {
    let is_shell = view.label() == "main";
    let window = window.clone();
    view.with_webview(move |platform| {
      if let Ok(container) = window.default_vbox() {
        let widget = platform.inner();
        // GtkBox honors WebKitGTK's natural height, not just size_request's
        // minimum. The shell can request a previous full-window height (890px
        // observed), even with expand/fill disabled. Cap its natural height in
        // a scrolled container so only the content receives remaining space.
        if is_shell {
          // GTK may insert a GtkViewport around a non-Scrollable webview.
          let frame = widget
            .ancestor(gtk::ScrolledWindow::static_type())
            .and_then(|parent| parent.downcast::<gtk::ScrolledWindow>().ok())
            .unwrap_or_else(|| {
              let frame = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::External)
                .vscrollbar_policy(gtk::PolicyType::External)
                .min_content_height(CHROME_HEIGHT as i32)
                .max_content_height(CHROME_HEIGHT as i32)
                .propagate_natural_height(true)
                .hexpand(true)
                .vexpand(false)
                .build();
              container.remove(&widget);
              frame.add(&widget);
              container.pack_start(&frame, false, false, 0);
              container.reorder_child(&frame, 0);
              frame
            });
          widget.set_size_request(-1, CHROME_HEIGHT as i32);
          if visible {
            // Also show GTK's automatically inserted viewport.
            frame.show_all();
          } else {
            frame.hide();
          }
        } else {
          container.set_child_packing(&widget, true, true, 0, gtk::PackType::Start);
          widget.set_vexpand(true);
        }
      }
    })?;
  }
  Ok(())
}

fn shortcut(app: &AppHandle, id: &str, url: &Url) {
  // A content page can request only the same limited operations as its keyboard
  // shortcuts, and only while it is the active tab. It cannot supply IDs/URLs or
  // invoke shell commands. No remote IPC permission is needed.
  if snapshot(app).active != id {
    return;
  }
  match url.host_str().unwrap_or_default() {
    "close" => send(app, Action::Close(id.into())),
    "new" => open(app, app.state::<DesktopTabs>().home.clone()),
    "next" => send(app, Action::Cycle(1)),
    "previous" => send(app, Action::Cycle(-1)),
    "duplicate" => send(app, Action::Duplicate(id.into())),
    "reopen" => send(app, Action::Reopen),
    "zoom-in" => send(app, Action::Zoom(10)),
    "zoom-out" => send(app, Action::Zoom(-10)),
    "zoom-reset" => send(app, Action::Zoom(0)),
    "move-left" => send(app, Action::MoveBy(id.into(), -1)),
    "move-right" => send(app, Action::MoveBy(id.into(), 1)),
    name if name.starts_with("select-") => {
      if let Ok(digit @ 1..=9) = name[7..].parse::<usize>() {
        send(app, Action::SelectDigit(digit));
      }
    }
    _ => {}
  }
}

fn create(
  app: &AppHandle,
  url: Url,
  next_id: &mut usize,
  insert_at: Option<usize>,
) -> tauri::Result<()> {
  *next_id += 1;
  let id = format!("page-{next_id}");
  let app_page = url.host_str().is_some_and(is_app_origin);
  let nav_app = app.clone();
  let nav_id = id.clone();
  let popup_app = app.clone();
  let popup_id = id.clone();
  let title_app = app.clone();
  let builder = WebviewBuilder::new(&id, WebviewUrl::External(url.clone()))
    .background_color(tauri::webview::Color(37, 37, 37, 255))
    .focused(false)
    .initialization_script(LOADING_SCRIPT)
    .initialization_script(desktop_info_script(app))
    .initialization_script(include_str!("../../dist/tab-keys.js"))
    .initialization_script(include_str!("../../dist/tab-shortcuts.js"))
    .on_navigation(move |url| {
      if url.scheme() == "whooing-tab" {
        shortcut(&nav_app, &nav_id, url);
        return false;
      }
      match link_action(url, false, app_page) {
        LinkAction::Stay => true,
        LinkAction::Tab => {
          send(&nav_app, Action::OpenAfter(url.clone(), nav_id.clone()));
          false
        }
        LinkAction::Browser => {
          open_in_browser(&nav_app, url);
          false
        }
        LinkAction::Block => false,
      }
    })
    .on_new_window(move |url, _| {
      match link_action(&url, true, false) {
        LinkAction::Tab => send(&popup_app, Action::OpenAfter(url, popup_id.clone())),
        LinkAction::Browser => open_in_browser(&popup_app, &url),
        _ => {}
      }
      NewWindowResponse::Deny
    })
    .on_document_title_changed(move |view, title| {
      if !title.trim().is_empty() {
        if let Some(tab) = title_app
          .state::<DesktopTabs>()
          .snapshot
          .lock()
          .unwrap()
          .tabs
          .iter_mut()
          .find(|tab| tab.id == view.label())
        {
          tab.title = title;
        }
        publish(&title_app);
      }
    });
  let tab = Tab {
    id: id.clone(),
    title: url.host_str().unwrap_or("Whooing").into(),
  };
  {
    let state = app.state::<DesktopTabs>();
    let mut state = state.snapshot.lock().unwrap();
    let index = insert_at.unwrap_or(state.tabs.len()).min(state.tabs.len());
    state.tabs.insert(index, tab);
  }
  let (position, size) = bounds(app)?;
  let window = app.get_window("main").ok_or(tauri::Error::WindowNotFound)?;
  let view = match window.add_child(builder, position, size) {
    Ok(view) => view,
    Err(error) => {
      app
        .state::<DesktopTabs>()
        .snapshot
        .lock()
        .unwrap()
        .tabs
        .retain(|tab| tab.id != id);
      publish(app);
      return Err(error);
    }
  };
  view.set_zoom(f64::from(*app.state::<DesktopTabs>().zoom_percent.lock().unwrap()) / 100.)?;
  #[cfg(target_os = "macos")]
  view.with_webview(|platform| {
    // with_webview runs on the main thread; this is the child WKWebView.
    unsafe {
      (&*platform.inner().cast::<objc2_web_kit::WKWebView>())
        .setAllowsBackForwardNavigationGestures(true);
    }
  })?;
  #[cfg(not(target_os = "macos"))]
  let _ = view;
  #[cfg(target_os = "linux")]
  for tab in snapshot(app).tabs {
    if tab.id != id {
      if let Some(view) = app.get_webview(&tab.id) {
        view.hide()?;
      }
    }
  }
  resize(app)?;
  select(app, &id)
}

fn numbered_tab(digit: usize, len: usize) -> Option<usize> {
  if len == 0 || !(1..=9).contains(&digit) {
    return None;
  }
  let index = if digit == 9 { len - 1 } else { digit - 1 };
  (index < len).then_some(index)
}

fn move_tab(tabs: &mut Vec<Tab>, id: &str, before: Option<&str>) {
  if before == Some(id) || before.is_some_and(|id| !tabs.iter().any(|tab| tab.id == id)) {
    return;
  }
  let Some(index) = tabs.iter().position(|tab| tab.id == id) else {
    return;
  };
  let tab = tabs.remove(index);
  let index = before
    .and_then(|id| tabs.iter().position(|tab| tab.id == id))
    .unwrap_or(tabs.len());
  tabs.insert(index, tab);
}

fn reusable_url(url: &Url) -> bool {
  matches!(url.scheme(), "http" | "https")
    && !must_open_in_browser(url)
    && url.username().is_empty()
    && url.password().is_none()
    && !url.path().starts_with("/auth/oauth")
    && !url.path().starts_with("/auth/security")
}

fn close_tabs(app: &AppHandle, ids: &[String]) -> tauri::Result<()> {
  let before = snapshot(app);
  let ids: Vec<_> = before
    .tabs
    .iter()
    .filter(|tab| ids.contains(&tab.id))
    .map(|tab| tab.id.clone())
    .collect();
  if ids.is_empty() {
    return Ok(());
  }
  if ids.len() == before.tabs.len() {
    if let Some(window) = app.get_window("main") {
      window.close()?;
    }
    return Ok(());
  }
  // Store batch closes right-to-left: LIFO reopen then restores their original order.
  for id in ids.iter().rev() {
    if let Some(view) = app.get_webview(id) {
      let restore = view.url().ok().filter(reusable_url);
      view.close()?;
      if let Some(url) = restore {
        let index = before.tabs.iter().position(|tab| tab.id == *id).unwrap();
        let state = app.state::<DesktopTabs>();
        let mut closed = state.closed.lock().unwrap();
        if closed.len() == 20 {
          closed.remove(0);
        }
        closed.push(ClosedTab { url, index });
      }
    }
    app
      .state::<DesktopTabs>()
      .snapshot
      .lock()
      .unwrap()
      .tabs
      .retain(|tab| tab.id != *id);
  }
  resize(app)?;
  let after = snapshot(app);
  if ids.contains(&before.active) {
    let old_index = before
      .tabs
      .iter()
      .position(|tab| tab.id == before.active)
      .unwrap_or(0);
    select(app, &after.tabs[old_index.min(after.tabs.len() - 1)].id)?;
  } else {
    publish(app);
  }
  Ok(())
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
  if let Some(action) = event.id().as_ref().strip_prefix("whooing-active:") {
    let id = snapshot(app).active;
    let action = match action {
      "new" => Action::Open(app.state::<DesktopTabs>().home.clone()),
      "duplicate" => Action::Duplicate(id),
      "close" => Action::Close(id),
      "reopen" => Action::Reopen,
      "reload" => Action::Reload(id),
      "next" => Action::Cycle(1),
      "previous" => Action::Cycle(-1),
      name if name.starts_with("select-") => {
        let Ok(digit @ 1..=9) = name[7..].parse::<usize>() else {
          return;
        };
        Action::SelectDigit(digit)
      }
      _ => return,
    };
    send(app, action);
    return;
  }
  let Some(value) = event.id().as_ref().strip_prefix("whooing-tab:") else {
    return;
  };
  let Some((action, id)) = value.split_once(':') else {
    return;
  };
  let action = match action {
    "duplicate" => Action::Duplicate(id.into()),
    "left" => Action::MoveBy(id.into(), -1),
    "right" => Action::MoveBy(id.into(), 1),
    "close" => Action::Close(id.into()),
    "others" => Action::CloseOthers(id.into()),
    "after" => Action::CloseRight(id.into()),
    "reopen" => Action::Reopen,
    _ => return,
  };
  send(app, action);
}

#[cfg(target_os = "macos")]
fn install_macos_menu(app: &tauri::App) -> tauri::Result<()> {
  use tauri::menu::{MenuItemKind, SubmenuBuilder};
  let menu = Menu::default(app.handle())?;
  // Keep App/Edit/View menus (including clipboard, undo, quit and fullscreen).
  // Replace the default Cmd+W host-window action with closing the active tab.
  for item in menu.items()? {
    if let MenuItemKind::Submenu(submenu) = item {
      match submenu.text()?.as_str() {
        "File" => {
          submenu.remove_at(0)?;
          for (name, title, shortcut) in [
            ("new", "새 후잉 탭", "Cmd+T"),
            ("duplicate", "탭 복제", "Cmd+Shift+D"),
            ("reopen", "닫은 탭 다시 열기", "Cmd+Shift+T"),
            ("close", "탭 닫기", "Cmd+W"),
            ("reload", "새로고침", "Cmd+R"),
          ] {
            submenu.append(
              &MenuItemBuilder::with_id(format!("whooing-active:{name}"), title)
                .accelerator(shortcut)
                .build(app)?,
            )?;
          }
        }
        "Window" => {
          let count = submenu.items()?.len();
          if count > 0 {
            submenu.remove_at(count - 1)?;
          }
        }
        _ => {}
      }
    }
  }
  let tabs = SubmenuBuilder::new(app, "탭")
    .item(
      &MenuItemBuilder::with_id("whooing-active:previous", "이전 탭")
        .accelerator("Cmd+Shift+[")
        .build(app)?,
    )
    .item(
      &MenuItemBuilder::with_id("whooing-active:next", "다음 탭")
        .accelerator("Cmd+Shift+]")
        .build(app)?,
    )
    .separator()
    .build()?;
  for digit in 1..=9 {
    let title = if digit == 9 {
      "마지막 탭".into()
    } else {
      format!("{digit}번째 탭")
    };
    tabs.append(
      &MenuItemBuilder::with_id(format!("whooing-active:select-{digit}"), title)
        .accelerator(format!("Cmd+{digit}"))
        .build(app)?,
    )?;
  }
  menu.append(&tabs)?;
  app.set_menu(menu)?;
  Ok(())
}

fn context_menu(app: &AppHandle, id: &str, x: f64, y: f64) -> tauri::Result<()> {
  let state = snapshot(app);
  let Some(index) = state.tabs.iter().position(|tab| tab.id == id) else {
    return Ok(());
  };
  let duplicable = app
    .get_webview(id)
    .and_then(|view| view.url().ok())
    .is_some_and(|url| reusable_url(&url));
  let item = |action: &str, title: &str, enabled: bool| {
    MenuItemBuilder::with_id(format!("whooing-tab:{action}:{id}"), title)
      .enabled(enabled)
      .build(app)
  };
  let menu = MenuBuilder::new(app)
    .item(&item("duplicate", "탭 복제", duplicable)?)
    .separator()
    .item(&item("left", "왼쪽으로 이동", index > 0)?)
    .item(&item(
      "right",
      "오른쪽으로 이동",
      index + 1 < state.tabs.len(),
    )?)
    .separator()
    .item(&item("close", "탭 닫기", true)?)
    .item(&item("others", "다른 탭 닫기", state.tabs.len() > 1)?)
    .item(&item(
      "after",
      "오른쪽 탭 닫기",
      index + 1 < state.tabs.len(),
    )?)
    .separator()
    .item(&item("reopen", "닫은 탭 다시 열기", state.can_reopen)?)
    .build()?;
  *app.state::<DesktopTabs>().context_menu.lock().unwrap() = Some(menu.clone());
  if let Some(window) = app.get_window("main") {
    window.popup_menu_at(&menu, LogicalPosition::new(x.max(0.), y.max(0.)))?;
  }
  Ok(())
}

fn perform(app: &AppHandle, action: Action, next_id: &mut usize) -> tauri::Result<()> {
  match action {
    Action::Open(url) => {
      let state = snapshot(app);
      let index = state
        .tabs
        .iter()
        .position(|tab| tab.id == state.active)
        .map(|index| index + 1);
      create(app, url, next_id, index)?;
    }
    Action::Select(id) => select(app, &id)?,
    Action::OpenAfter(url, id) => {
      let state = snapshot(app);
      let index = state
        .tabs
        .iter()
        .position(|tab| tab.id == id)
        .map(|index| index + 1);
      create(app, url, next_id, index)?;
    }
    Action::Close(id) => {
      close_tabs(app, &[id])?;
    }
    Action::Cycle(delta) => {
      let state = snapshot(app);
      if let Some(index) = state.tabs.iter().position(|tab| tab.id == state.active) {
        let index = (index as i32 + delta).rem_euclid(state.tabs.len() as i32) as usize;
        select(app, &state.tabs[index].id)?;
      }
    }
    Action::Reload(id) => {
      if let Some(view) = app.get_webview(&id) {
        view.reload()?;
      }
    }
    Action::Zoom(delta) => {
      let percent = {
        let state = app.state::<DesktopTabs>();
        let mut percent = state.zoom_percent.lock().unwrap();
        let next = next_zoom(*percent, delta);
        save_zoom(&app.path().app_config_dir()?.join("zoom.json"), next)?;
        *percent = next;
        next
      };
      for tab in snapshot(app).tabs {
        if let Some(view) = app.get_webview(&tab.id) {
          view.set_zoom(f64::from(percent) / 100.)?;
        }
      }
    }
    Action::Resize => {
      resize(app)?;
    }
    Action::SelectDigit(digit) => {
      let state = snapshot(app);
      if let Some(index) = numbered_tab(digit, state.tabs.len()) {
        select(app, &state.tabs[index].id)?;
      }
    }
    Action::Duplicate(id) => {
      let state = snapshot(app);
      if let (Some(index), Some(view)) = (
        state.tabs.iter().position(|tab| tab.id == id),
        app.get_webview(&id),
      ) {
        let url = view.url()?;
        if reusable_url(&url) {
          create(app, url, next_id, Some(index + 1))?;
        }
      }
    }
    Action::Move { id, before } => {
      move_tab(
        &mut app.state::<DesktopTabs>().snapshot.lock().unwrap().tabs,
        &id,
        before.as_deref(),
      );
      publish(app);
    }
    Action::MoveBy(id, delta) => {
      let state = snapshot(app);
      if let Some(index) = state.tabs.iter().position(|tab| tab.id == id) {
        if delta < 0 && index > 0 {
          move_tab(
            &mut app.state::<DesktopTabs>().snapshot.lock().unwrap().tabs,
            &id,
            Some(&state.tabs[index - 1].id),
          );
          publish(app);
        } else if delta > 0 && index + 1 < state.tabs.len() {
          let before = state.tabs.get(index + 2).map(|tab| tab.id.as_str());
          move_tab(
            &mut app.state::<DesktopTabs>().snapshot.lock().unwrap().tabs,
            &id,
            before,
          );
          publish(app);
        }
      }
    }
    Action::Reopen => {
      let closed = app.state::<DesktopTabs>().closed.lock().unwrap().pop();
      if let Some(closed) = closed {
        if let Err(error) = create(app, closed.url.clone(), next_id, Some(closed.index)) {
          app
            .state::<DesktopTabs>()
            .closed
            .lock()
            .unwrap()
            .push(closed);
          return Err(error);
        }
      }
    }
    Action::CloseOthers(id) => {
      let state = snapshot(app);
      if state.tabs.iter().any(|tab| tab.id == id) {
        close_tabs(
          app,
          &state
            .tabs
            .iter()
            .filter(|tab| tab.id != id)
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>(),
        )?;
      }
    }
    Action::CloseRight(id) => {
      let state = snapshot(app);
      if let Some(index) = state.tabs.iter().position(|tab| tab.id == id) {
        close_tabs(
          app,
          &state.tabs[index + 1..]
            .iter()
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>(),
        )?;
      }
    }
    Action::History(id, forward) => {
      if let Some(view) = app.get_webview(&id) {
        view.eval(if forward {
          "history.forward()"
        } else {
          "history.back()"
        })?;
      }
    }
    Action::Context(id, x, y) => context_menu(app, &id, x, y)?,
    Action::DeepLink(url) => {
      let state = snapshot(app);
      let home_origin = app.state::<DesktopTabs>().home.origin();
      let home = state.tabs.iter().find(|tab| {
        app
          .get_webview(&tab.id)
          .and_then(|view| view.url().ok())
          .is_some_and(|url| url.origin() == home_origin)
      });
      if let Some(tab) = home {
        if let Some(view) = app.get_webview(&tab.id) {
          view.navigate(url)?;
          select(app, &tab.id)?;
        }
      } else {
        create(app, url, next_id, None)?;
      }
      if let Some(window) = app.get_window("main") {
        window.unminimize()?;
        window.set_focus()?;
      }
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn app_zoom_survives_reload_and_reset_and_rejects_invalid_preferences() {
    let directory = std::env::temp_dir().join(format!("whooing-zoom-test-{}", std::process::id()));
    let path = directory.join("zoom.json");
    assert_eq!(load_zoom(&path), 100);
    save_zoom(&path, next_zoom(100, 10)).unwrap();
    assert_eq!(load_zoom(&path), 110);
    save_zoom(&path, next_zoom(load_zoom(&path), -10)).unwrap();
    assert_eq!(load_zoom(&path), 100);
    save_zoom(&path, next_zoom(150, 0)).unwrap();
    assert_eq!(load_zoom(&path), 100);
    assert_eq!(next_zoom(50, -10), 50);
    assert_eq!(next_zoom(300, 10), 300);
    for invalid in ["broken", "null", "49", "301", "110.5"] {
      std::fs::write(&path, invalid).unwrap();
      assert_eq!(load_zoom(&path), 100);
    }
    std::fs::remove_dir_all(directory).unwrap();
  }

  #[cfg(target_os = "linux")]
  #[test]
  fn linux_ctrl_tab_handles_shifted_keysyms_without_stealing_other_shortcuts() {
    use gtk::gdk::{keys::constants as key, ModifierType as mods};
    assert_eq!(linux_tab_cycle(key::Tab, mods::CONTROL_MASK), Some(1));
    assert_eq!(linux_tab_cycle(key::Tab, mods::CONTROL_MASK | mods::SHIFT_MASK), Some(-1));
    assert_eq!(linux_tab_cycle(key::ISO_Left_Tab, mods::CONTROL_MASK), Some(-1));
    assert_eq!(linux_tab_cycle(key::ISO_Left_Tab, mods::CONTROL_MASK | mods::SHIFT_MASK), Some(-1));
    assert_eq!(linux_tab_cycle(key::KP_Tab, mods::CONTROL_MASK | mods::LOCK_MASK), Some(1));
    assert_eq!(linux_tab_cycle(key::Tab, mods::empty()), None);
    assert_eq!(linux_tab_cycle(key::ISO_Left_Tab, mods::SHIFT_MASK), None);
    for extra in [mods::MOD1_MASK, mods::SUPER_MASK, mods::META_MASK] {
      assert_eq!(linux_tab_cycle(key::Tab, mods::CONTROL_MASK | extra), None);
    }
    assert_eq!(linux_tab_cycle(key::t, mods::CONTROL_MASK), None);
  }

  #[test]
  fn numbers_select_positions_and_nine_always_selects_last() {
    assert_eq!(numbered_tab(1, 12), Some(0));
    assert_eq!(numbered_tab(8, 12), Some(7));
    assert_eq!(numbered_tab(9, 12), Some(11));
    assert_eq!(numbered_tab(9, 2), Some(1));
    for (digit, len) in [(0, 3), (10, 12), (8, 2), (1, 0), (9, 0)] {
      assert_eq!(numbered_tab(digit, len), None);
    }
  }

  #[test]
  fn reordering_recalculates_the_target_after_removing_the_source() {
    let mut tabs: Vec<_> = ["a", "b", "c", "d"]
      .into_iter()
      .map(|id| Tab {
        id: id.into(),
        title: id.into(),
      })
      .collect();
    let ids = |tabs: &[Tab]| tabs.iter().map(|tab| tab.id.as_str()).collect::<String>();
    move_tab(&mut tabs, "a", Some("d"));
    assert_eq!(ids(&tabs), "bcad");
    move_tab(&mut tabs, "d", Some("b"));
    assert_eq!(ids(&tabs), "dbca");
    move_tab(&mut tabs, "b", None);
    assert_eq!(ids(&tabs), "dcab");
    move_tab(&mut tabs, "d", Some("d"));
    move_tab(&mut tabs, "d", Some("missing"));
    move_tab(&mut tabs, "missing", None);
    assert_eq!(ids(&tabs), "dcab");
  }

  #[test]
  fn duplication_and_reopen_never_replay_auth_handoffs() {
    for url in [
      "https://q22.whooing.com/report?month=2026-10",
      "https://example.com/path?filter=saved#section",
      "https://whooing.com/auth/login",
    ] {
      assert!(reusable_url(&url.parse().unwrap()), "{url}");
    }
    for url in [
      "https://q22.whooing.com/auth/oauth/google",
      "https://q22.whooing.com/auth/oauth_deeplink/receive?code=test",
      "https://q22.whooing.com/auth/security",
      "https://accounts.google.com/o/oauth2/auth",
      "https://gumoisland.com/redirect/export_to/go",
      "https://user:password@example.com/",
      "mailto:test@example.com",
      "about:blank",
      "file:///test",
    ] {
      assert!(!reusable_url(&url.parse().unwrap()), "{url}");
    }
  }

  #[test]
  fn frontend_tab_dto_exposes_only_id_and_title() {
    let tab = Tab {
      id: "page-1".into(),
      title: "Report".into(),
    };
    assert_eq!(
      serde_json::to_value(tab).unwrap(),
      serde_json::json!({"id":"page-1", "title":"Report"})
    );
  }
}
