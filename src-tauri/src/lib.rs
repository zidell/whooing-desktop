use tauri::{Manager, UserAttentionType, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;

const APP_ORIGIN_HOST: &str = "whooing.com";
const LOADING_SCRIPT: &str = include_str!("../../dist/loading.js");

fn is_external_url(url: &tauri::Url) -> bool {
  matches!(url.scheme(), "http" | "https" | "mailto")
}

fn is_internal_url(url: &tauri::Url) -> bool {
  url.as_str() == "about:blank"
    || (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
    || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"))
    || is_dev_server_url(url)
}

// `tauri dev`는 devUrl이 없으면 dist를 내장 개발 서버(http://127.0.0.1:<포트>)로 띄운다.
// 릴리즈 빌드에는 없는 주소라 개발 빌드에서만 내부로 본다.
fn is_dev_server_url(url: &tauri::Url) -> bool {
  cfg!(debug_assertions) && url.scheme() == "http" && url.host_str() == Some("127.0.0.1")
}

// 원격 whooing.com 페이지에 주입되는 스크립트. window.open()과 target="_blank" 링크를
// 앱 내부에서 처리하지 않고 open_external 커맨드를 통해 시스템 기본 브라우저로 넘긴다.
// macOS는 이 스크립트 대신 tabs 모듈이 웹뷰의 새 창 요청을 받아 앱 탭으로 연다.
#[cfg(not(target_os = "macos"))]
const EXTERNAL_LINK_SCRIPT: &str = r#"
(function () {
  function openExternal(url) {
    if (!url) return;
    try {
      var resolved = new URL(url, window.location.href);
      if (!['http:', 'https:', 'mailto:'].includes(resolved.protocol)) return;
      window.__TAURI__.core.invoke('open_external', { url: resolved.href }).catch(console.error);
    } catch (_) {}
  }
  var nativeOpen = window.open;
  window.open = function (url) {
    if (url) {
      openExternal(url);
      return null;
    }
    return nativeOpen.apply(window, arguments);
  };
  document.addEventListener('click', function (e) {
    var a = e.target && e.target.closest && e.target.closest('a[target="_blank"]');
    if (a && a.href) {
      e.preventDefault();
      openExternal(a.href);
    }
  }, true);
})();
"#;

// Tauri는 Electron과 달리 기본 앱 메뉴/새로고침 단축키를 제공하지 않고,
// 임베드 웹뷰(WKWebView/WebView2/WebKitGTK)도 브라우저 크롬 없이는 Ctrl+R/Cmd+R을
// 자체적으로 바인딩하지 않는다(Windows WebView2도 실측 결과 동작 안 함). 3개 OS 공통으로
// 새로고침을 보장하기 위해 직접 키 리스너를 주입한다.
const RELOAD_SHORTCUT_SCRIPT: &str = r#"
(function () {
  document.addEventListener('keydown', function (e) {
    var key = e.key ? e.key.toLowerCase() : '';
    if ((e.metaKey || e.ctrlKey) && key === 'r') {
      e.preventDefault();
      e.stopPropagation();
      window.location.reload();
    }
  }, true);
})();
"#;

fn is_app_origin(host: &str) -> bool {
  host == APP_ORIGIN_HOST || host.ends_with(&format!(".{APP_ORIGIN_HOST}"))
}

// 앱 탭이 아니라 기본 브라우저에서 진행해야 하는 주소. SNS 로그인은 제공자가 임베드 웹뷰를
// 막고(구글 disallowed_useragent 등) 딥링크로 앱에 돌아오는 구조이고, 결제는 웹이
// gumoisland.com의 export_to 발판으로 "외부 브라우저로 보내라"는 뜻을 표시해 넘긴다.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn must_open_in_browser(url: &tauri::Url) -> bool {
  if !matches!(url.scheme(), "http" | "https") {
    return true;
  }
  let host = url.host_str().unwrap_or_default();
  let path = url.path();
  if is_app_origin(host) {
    return path.starts_with("/auth/oauth/");
  }
  match host {
    "accounts.google.com" | "appleid.apple.com" | "nid.naver.com" => true,
    "gumoisland.com" | "www.gumoisland.com" => path.starts_with("/redirect/export_to/"),
    _ => {
      (host == "facebook.com" || host.ends_with(".facebook.com")) && path.contains("/dialog/oauth")
    }
  }
}

fn open_in_browser(app: &tauri::AppHandle, url: &tauri::Url) {
  if is_external_url(url) {
    let _ = app.opener().open_url(url.as_str(), None::<&str>);
  }
}

// 웹뷰 밖으로 나가는 링크. macOS는 SNS 로그인·결제만 기본 브라우저로 보내고 나머지는
// 링크를 연 창에 붙는 네이티브 탭으로 연다. 다른 OS는 전부 기본 브라우저로 보낸다.
fn open_outside_webview(app: &tauri::AppHandle, opener_label: &str, url: &tauri::Url) {
  #[cfg(target_os = "macos")]
  if !must_open_in_browser(url) {
    tabs::open(app, opener_label, url.clone());
    return;
  }
  let _ = opener_label;
  open_in_browser(app, url);
}

// whooing.com 쪽 JS가 "데스크톱 앱에서 열렸는지, 몇 버전인지"를 판별할 수 있도록
// 전역 변수로 노출한다(예: 추후 강제 업데이트 안내 등에 활용 가능). 이 변수의
// 존재 여부 자체가 곧 "타우리 데스크톱 앱 여부" 판별 기준이 된다. platform은
// navigator.userAgent에 이미 있으므로 중복 노출하지 않는다. 앱 탭에서 여는 외부 사이트에는
// 알릴 이유가 없어 whooing.com에서만 정의한다.
fn desktop_info_script(app: &tauri::AppHandle) -> String {
  format!(
    "if (location.hostname === '{APP_ORIGIN_HOST}' || location.hostname.endsWith('.{APP_ORIGIN_HOST}')) \
     window.__WHOOING_DESKTOP__ = '{}';",
    app.package_info().version
  )
}

// 메인 창이 닫혀도 탭이 남아 있으면 그 탭이 딥링크·재실행 포커스를 받는다.
fn primary_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
  app
    .get_webview_window("main")
    .or_else(|| app.webview_windows().into_values().next())
}

// whooing://<path>?<query> 형태의 딥링크(예: OAuth 콜백 핸드오프)를
// https://whooing.com/<path>?<query> 로 변환해 메인 윈도우를 이동시킨다.
fn handle_deep_link_url(app: &tauri::AppHandle, url: &tauri::Url) {
  let Some(window) = primary_window(app) else {
    return;
  };
  if let Some(target) = deep_link_target(url) {
    let _ = window.navigate(target);
  }
  let _ = window.set_focus();
}

fn deep_link_target(url: &tauri::Url) -> Option<tauri::Url> {
  if url.scheme() != "whooing" {
    return None;
  }
  // whooing://auth/oauth_deeplink/... 형태는 "auth"가 path가 아니라 host로 파싱되므로
  // (예: whooing://auth/... -> host="auth", path="/..."), host를 다시 path 앞에 붙여야
  // 원래 경로(/auth/oauth_deeplink/...)가 복원된다.
  let host = url.host_str().unwrap_or_default();
  let mut target = if host.is_empty() {
    format!("https://{APP_ORIGIN_HOST}{}", url.path())
  } else {
    format!("https://{APP_ORIGIN_HOST}/{host}{}", url.path())
  };
  if let Some(query) = url.query() {
    target.push('?');
    target.push_str(query);
  }
  target.parse().ok()
}

#[tauri::command]
fn set_notification_badge(window: tauri::WebviewWindow, count: i64) -> Result<(), String> {
  // macOS(독 숫자 뱃지) / Linux(libunity 지원 환경). Windows는 Tauri에서 숫자 뱃지 미지원.
  window
    .set_badge_count(if count > 0 { Some(count) } else { None })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn notify_new_message(window: tauri::WebviewWindow) -> Result<(), String> {
  // macOS: 독 아이콘 한 번 튕김 / Windows: 포커스 잡을 때까지 작업표시줄 깜빡임.
  window
    .request_user_attention(Some(UserAttentionType::Informational))
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
  let parsed = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
  if !is_external_url(&parsed) {
    return Err("Unsupported external URL scheme".into());
  }
  app
    .opener()
    .open_url(parsed.as_str(), None::<&str>)
    .map_err(|e| e.to_string())
}

// macOS 전용 앱 내 탭. 링크를 새 WebviewWindow로 만들어 링크를 연 창의 네이티브 탭 그룹에
// 붙인다(NSWindow addTabbedWindow). 탭바·⌘W·⌃Tab·탭 분리는 macOS가 처리한다.
// 시스템 설정 "문서를 열 때 탭 선호"와 무관하게 항상 탭으로 붙이려고 직접 붙인다.
#[cfg(target_os = "macos")]
mod tabs {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use objc2_app_kit::{NSWindow, NSWindowOrderingMode};
  use objc2_web_kit::WKWebView;
  use tauri::webview::NewWindowResponse;
  use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

  use super::{
    desktop_info_script, is_app_origin, is_external_url, must_open_in_browser, open_in_browser,
    open_outside_webview, LOADING_SCRIPT, RELOAD_SHORTCUT_SCRIPT,
  };

  pub const TABBING_IDENTIFIER: &str = "whooing";
  static NEXT_TAB: AtomicUsize = AtomicUsize::new(1);

  // 탭마다 방문 기록은 있지만 웹뷰에 뒤로·앞으로 단축키가 없어서 사파리와 같은 ⌘[ / ⌘]를 붙인다.
  pub const HISTORY_SHORTCUT_SCRIPT: &str = r#"
(function () {
  document.addEventListener('keydown', function (e) {
    if (!e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
    if (e.key === '[') {
      e.preventDefault();
      history.back();
    } else if (e.key === ']') {
      e.preventDefault();
      history.forward();
    }
  }, true);
})();
"#;

  // 트랙패드 두 손가락 좌우 스와이프로 뒤로·앞으로(WKWebView 기본값은 꺼짐, Tauri는 옵션을 노출하지 않는다).
  pub fn enable_swipe_navigation(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| {
      // Safety: macOS에서 PlatformWebview::inner()는 이 창의 WKWebView다. with_webview는 메인 스레드에서 실행한다.
      unsafe {
        let view = &*webview.inner().cast::<WKWebView>();
        view.setAllowsBackForwardNavigationGestures(true);
      }
    });
  }

  // 웹뷰의 새 창 요청(target="_blank", window.open). 앱이 직접 만든 탭은 opener 관계가 없어
  // window.open()은 null을 받는다(옛 주입 스크립트와 같다).
  pub fn on_new_window(app: &AppHandle, opener_label: &str, url: Url) -> NewWindowResponse<tauri::Wry> {
    if matches!(url.scheme(), "http" | "https") {
      open_outside_webview(app, opener_label, &url);
    } else if is_external_url(&url) {
      open_in_browser(app, &url);
    }
    NewWindowResponse::Deny
  }

  pub fn set_title_from_document(window: WebviewWindow, title: String) {
    if !title.trim().is_empty() {
      let _ = window.set_title(&title);
    }
  }

  // 웹뷰 콜백(메인 스레드) 안에서 창을 만들지 않도록 비동기로 넘긴다.
  pub fn open(app: &AppHandle, opener_label: &str, url: Url) {
    let app = app.clone();
    let opener_label = opener_label.to_string();
    tauri::async_runtime::spawn(async move {
      if let Err(error) = create(&app, &opener_label, url) {
        log::warn!("Opening a tab failed: {error}");
      }
    });
  }

  fn create(app: &AppHandle, opener_label: &str, url: Url) -> tauri::Result<()> {
    let label = format!("tab-{}", NEXT_TAB.fetch_add(1, Ordering::Relaxed));
    // whooing.com에서 시작한 탭은 메인 창처럼 앱 밖 주소를 다시 탭으로 넘기고,
    // 외부 사이트 탭은 그 사이트 안의 이동을 그대로 둔다.
    let app_page = url.host_str().is_some_and(is_app_origin);
    let new_window_app = app.clone();
    let new_window_label = label.clone();
    let navigation_app = app.clone();
    let navigation_label = label.clone();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
      .title(app.config().product_name.as_deref().unwrap_or("Whooing"))
      .inner_size(1280.0, 800.0)
      .min_inner_size(960.0, 600.0)
      .resizable(true)
      .visible(false)
      .tabbing_identifier(TABBING_IDENTIFIER)
      .initialization_script(LOADING_SCRIPT)
      .initialization_script(desktop_info_script(app))
      .initialization_script(RELOAD_SHORTCUT_SCRIPT)
      .initialization_script(HISTORY_SHORTCUT_SCRIPT)
      .on_document_title_changed(set_title_from_document)
      .on_new_window(move |url, _features| on_new_window(&new_window_app, &new_window_label, url))
      .on_navigation(move |url| {
        if !matches!(url.scheme(), "http" | "https") {
          // 외부 사이트의 about:blank·data:·blob: 프레임은 그대로 두고 mailto만 넘긴다.
          if is_external_url(url) {
            open_in_browser(&navigation_app, url);
            return false;
          }
          return true;
        }
        if url.host_str().is_some_and(is_app_origin) {
          return true;
        }
        if app_page {
          open_outside_webview(&navigation_app, &navigation_label, url);
          return false;
        }
        if must_open_in_browser(url) {
          open_in_browser(&navigation_app, url);
          return false;
        }
        true
      })
      .build()?;
    enable_swipe_navigation(&window);

    let opener = app.get_webview_window(opener_label);
    app.run_on_main_thread(move || {
      if let Some(opener) = opener {
        attach(&opener, &window);
      }
      let _ = window.show();
      let _ = window.set_focus();
    })
  }

  fn attach(opener: &WebviewWindow, tab: &WebviewWindow) {
    let (Ok(parent), Ok(child)) = (opener.ns_window(), tab.ns_window()) else {
      return;
    };
    // Safety: 둘 다 살아 있는 Tauri 창의 NSWindow이고, run_on_main_thread 안에서만 부른다.
    unsafe {
      let parent = &*parent.cast::<NSWindow>();
      let child = &*child.cast::<NSWindow>();
      parent.addTabbedWindow_ordered(child, NSWindowOrderingMode::Above);
    }
  }
}

// macOS 전용 자동 업데이트. 윈도우는 MS Store가, 리눅스 deb/rpm은 패키지 관리자가 맡고
// Tauri 업데이터는 그 형식을 지원하지 않으므로 맥 빌드에만 넣는다.
// 시작 직후와 이후 주기적으로 latest.json을 확인해 새 버전을 백그라운드로 받아 설치하고,
// 설치가 끝나면 재시작 여부를 묻는다. "나중에"를 고르면 이미 교체된 번들이 다음 실행 때 뜬다.
#[cfg(target_os = "macos")]
mod auto_update {
  use std::time::Duration;
  use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
  use tauri_plugin_updater::UpdaterExt;

  const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
  const STARTUP_DELAY: Duration = Duration::from_secs(10);

  pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
      std::thread::sleep(STARTUP_DELAY);
      loop {
        match tauri::async_runtime::block_on(install_if_available(&app)) {
          Ok(Some(version)) => {
            // 설치가 끝나면 실행 중인 버전은 그대로라 다시 확인하면 같은 업데이트를 또 받는다.
            // 한 번 설치했으면 루프를 끝낸다.
            ask_restart(&app, &version);
            return;
          }
          Ok(None) => {}
          Err(error) => log::warn!("Auto update failed: {error}"),
        }
        std::thread::sleep(CHECK_INTERVAL);
      }
    });
  }

  async fn install_if_available(
    app: &tauri::AppHandle,
  ) -> Result<Option<String>, tauri_plugin_updater::Error> {
    let Some(update) = app.updater()?.check().await? else {
      return Ok(None);
    };
    let version = update.version.clone();
    update.download_and_install(|_, _| {}, || {}).await?;
    Ok(Some(version))
  }

  fn ask_restart(app: &tauri::AppHandle, version: &str) {
    let korean = sys_locale::get_locale().is_some_and(|locale| locale.starts_with("ko"));
    let (title, message, restart, later) = if korean {
      (
        "업데이트 설치됨".to_string(),
        format!("후잉 {version} 버전을 설치했어요. 지금 다시 시작할까요?"),
        "다시 시작".to_string(),
        "나중에".to_string(),
      )
    } else {
      (
        "Update installed".to_string(),
        format!("Whooing {version} has been installed. Restart now?"),
        "Restart".to_string(),
        "Later".to_string(),
      )
    };
    let confirmed = app
      .dialog()
      .message(message)
      .title(title)
      .kind(MessageDialogKind::Info)
      .buttons(MessageDialogButtons::OkCancelCustom(restart, later))
      .blocking_show();
    if confirmed {
      app.restart();
    }
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let mut builder = tauri::Builder::default();

  // 싱글 인스턴스 플러그인은 반드시 제일 먼저 등록해야 한다.
  // Windows/Linux는 macOS와 달리 딥링크를 OS 이벤트가 아니라 "새 인스턴스 실행(argv)"로
  // 전달하는데, deep-link feature가 이 argv를 감지해 기존 인스턴스의
  // deep_link().on_open_url() 이벤트로 그대로 넘겨준다.
  #[cfg(desktop)]
  {
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
      if let Some(window) = primary_window(app) {
        let _ = window.set_focus();
      }
    }));
  }

  #[cfg(target_os = "macos")]
  {
    builder = builder
      .plugin(tauri_plugin_updater::Builder::new().build())
      .plugin(tauri_plugin_dialog::init());
  }

  builder
    // 플러그인 기본값은 target="_blank" 링크 클릭을 가로채 plugin:opener|open_url을 부르는데,
    // 원격 whooing.com에는 opener 권한을 주지 않아 호출이 거부되고 클릭만 막힌다.
    // 새 창 요청은 macOS는 tabs 모듈, 다른 OS는 EXTERNAL_LINK_SCRIPT가 처리한다.
    .plugin(
      tauri_plugin_opener::Builder::new()
        .open_js_links_on_click(false)
        .build(),
    )
    .plugin(tauri_plugin_deep_link::init())
    .invoke_handler(tauri::generate_handler![
      set_notification_badge,
      notify_new_message,
      open_external
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // 콜드 스타트 딥링크는 윈도우가 생성되기 전에 초기 목적지로 저장한다.
      let start_url = app
        .deep_link()
        .get_current()
        .ok()
        .flatten()
        .and_then(|urls| urls.iter().find_map(deep_link_target))
        .unwrap_or_else(|| format!("https://{APP_ORIGIN_HOST}").parse().unwrap());

      let deep_link_app_handle = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
          handle_deep_link_url(&deep_link_app_handle, &url);
        }
      });

      let desktop_info_script = desktop_info_script(app.handle());

      // whooing.com(및 서브도메인) 외 도메인으로의 네비게이션은 임베드 웹뷰 안에서
      // 처리하지 않는다. macOS는 앱 탭으로, 다른 OS는 시스템 기본 브라우저로 넘긴다.
      // 구글 로그인 등 SNS 로그인과 결제는 어느 OS든 기본 브라우저로 간다.
      let navigation_app_handle = app.handle().clone();
      let mut main_window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title(app.config().product_name.as_deref().unwrap_or("Whooing"))
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 600.0)
        .resizable(true)
        .initialization_script(format!(
          "window.__WHOOING_START_URL__ = {};",
          serde_json::to_string(start_url.as_str())?
        ))
        .initialization_script(LOADING_SCRIPT)
        .initialization_script(desktop_info_script)
        .initialization_script(RELOAD_SHORTCUT_SCRIPT)
        .on_navigation(move |url| {
          if is_internal_url(url) {
            return true;
          }
          if matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(is_app_origin) {
            return true;
          }
          if is_external_url(url) {
            open_outside_webview(&navigation_app_handle, "main", url);
          }
          false
        });

      // macOS는 새 창 요청을 웹뷰 네이티브 콜백으로 받아 탭으로 열고,
      // 다른 OS는 주입 스크립트가 가로채 open_external로 기본 브라우저에 넘긴다.
      #[cfg(target_os = "macos")]
      {
        let new_window_app_handle = app.handle().clone();
        main_window = main_window
          .tabbing_identifier(tabs::TABBING_IDENTIFIER)
          .initialization_script(tabs::HISTORY_SHORTCUT_SCRIPT)
          .on_document_title_changed(tabs::set_title_from_document)
          .on_new_window(move |url, _features| {
            tabs::on_new_window(&new_window_app_handle, "main", url)
          });
      }
      #[cfg(not(target_os = "macos"))]
      {
        main_window = main_window.initialization_script(EXTERNAL_LINK_SCRIPT);
      }

      let main_window = main_window.build()?;
      #[cfg(target_os = "macos")]
      tabs::enable_swipe_navigation(&main_window);
      #[cfg(not(target_os = "macos"))]
      let _ = main_window;

      // 스킴 등록의 외부 프로세스 실행이 첫 화면 표시를 막지 않게 한다.
      // Linux 로컬 빌드는 설치된 앱의 OAuth 핸들러를 덮어쓰지 않는다.
      #[cfg(any(
        all(target_os = "linux", not(debug_assertions)),
        all(debug_assertions, windows)
      ))]
      {
        let registration_app = app.handle().clone();
        std::thread::spawn(move || {
          if let Err(error) = registration_app.deep_link().register_all() {
            log::warn!("Deep-link registration failed: {error}");
          }
        });
      }

      #[cfg(all(target_os = "macos", not(debug_assertions)))]
      auto_update::start(app.handle().clone());

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn internal_browser_pages_are_never_external_links() {
    for url in [
      "about:blank",
      "about:config",
      "javascript:alert(1)",
      "data:text/html,test",
      "file:///tmp/test",
    ] {
      assert!(!is_external_url(&url.parse().unwrap()), "{url}");
    }
    for url in [
      "https://example.com",
      "http://example.com",
      "mailto:test@example.com",
    ] {
      assert!(is_external_url(&url.parse().unwrap()), "{url}");
    }
  }

  #[test]
  fn local_loading_page_and_blank_webview_are_internal() {
    for url in [
      "about:blank",
      "tauri://localhost/index.html",
      "http://tauri.localhost/index.html",
      "https://tauri.localhost/index.html",
    ] {
      assert!(is_internal_url(&url.parse().unwrap()), "{url}");
    }
    for url in [
      "about:config",
      "https://localhost",
      "https://tauri.localhost.example.com",
    ] {
      assert!(!is_internal_url(&url.parse().unwrap()), "{url}");
    }
  }

  #[test]
  fn app_domain_matching_requires_a_domain_boundary() {
    assert!(is_app_origin("whooing.com"));
    assert!(is_app_origin("static.whooing.com"));
    assert!(!is_app_origin("notwhooing.com"));
    assert!(!is_app_origin("whooing.com.example.com"));
  }

  #[test]
  fn sns_login_and_payment_stay_in_the_default_browser() {
    for url in [
      "https://whooing.com/auth/oauth/google?go_to=todesktop&app_nonce=n",
      "https://accounts.google.com/o/oauth2/v2/auth?client_id=x",
      "https://appleid.apple.com/auth/authorize",
      "https://nid.naver.com/oauth2.0/authorize",
      "https://www.facebook.com/v18.0/dialog/oauth?client_id=x",
      "https://gumoisland.com/redirect/export_to/go?url=https%3A%2F%2Fwhooing.com%2Ftools%2Fpayment%2Ft",
      "mailto:test@example.com",
    ] {
      assert!(must_open_in_browser(&url.parse().unwrap()), "{url}");
    }
    for url in [
      "https://whooing.com/report",
      "https://whooing.com/auth/login",
      "https://www.facebook.com/whooing",
      "https://gumoisland.com/",
      "https://example.com/oauth/",
    ] {
      assert!(!must_open_in_browser(&url.parse().unwrap()), "{url}");
    }
  }

  #[test]
  fn cold_start_oauth_destination_preserves_path_and_query() {
    let url = "whooing://auth/oauth_deeplink/google?code=test&state=state"
      .parse()
      .unwrap();
    assert_eq!(
      deep_link_target(&url).unwrap().as_str(),
      "https://whooing.com/auth/oauth_deeplink/google?code=test&state=state"
    );
    assert!(deep_link_target(&"https://example.com".parse().unwrap()).is_none());
  }
}
