use tauri::{Manager, UserAttentionType, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;

const APP_ORIGIN_HOST: &str = "whooing.com";

// 원격 whooing.com 페이지에 주입되는 스크립트. window.open()과 target="_blank" 링크를
// 앱 내부에서 처리하지 않고 open_external 커맨드를 통해 시스템 기본 브라우저로 넘긴다.
const EXTERNAL_LINK_SCRIPT: &str = r#"
(function () {
  function openExternal(url) {
    if (!url) return;
    window.__TAURI__.core.invoke('open_external', { url: url });
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

fn is_app_origin(host: &str) -> bool {
  host == APP_ORIGIN_HOST || host.ends_with(&format!(".{APP_ORIGIN_HOST}"))
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
  app
    .opener()
    .open_url(url, None::<&str>)
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
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

      // 개발 모드 Linux/Windows에서는 커스텀 스킴이 자동 등록 안 되므로 수동 등록.
      // macOS는 번들 Info.plist(tauri.conf.json plugins.deep-link 설정)로만 등록 가능.
      #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
      app.deep_link().register_all()?;

      // whooing://<path>?<query> 형태의 딥링크(예: OAuth 콜백 핸드오프)를
      // https://whooing.com/<path>?<query> 로 변환해 메인 윈도우를 이동시킨다.
      let deep_link_app_handle = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        let Some(url) = event.urls().into_iter().next() else {
          return;
        };
        let Some(window) = deep_link_app_handle.get_webview_window("main") else {
          return;
        };
        let mut target = format!("https://{APP_ORIGIN_HOST}{}", url.path());
        if let Some(query) = url.query() {
          target.push('?');
          target.push_str(query);
        }
        if let Ok(parsed) = target.parse() {
          let _ = window.navigate(parsed);
        }
        let _ = window.set_focus();
      });

      // whooing.com(및 서브도메인) 외 도메인으로의 네비게이션은 임베드 웹뷰 안에서
      // 처리하지 않고 시스템 기본 브라우저로 넘긴다(구글 로그인 등 외부 OAuth 포함).
      let navigation_app_handle = app.handle().clone();
      WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::External(format!("https://{APP_ORIGIN_HOST}").parse().unwrap()),
      )
      .title("Whooing")
      .inner_size(1280.0, 800.0)
      .min_inner_size(960.0, 600.0)
      .resizable(true)
      .initialization_script(EXTERNAL_LINK_SCRIPT)
      .on_navigation(move |url| match url.host_str() {
        Some(host) if is_app_origin(host) => true,
        _ => {
          let _ = navigation_app_handle.opener().open_url(url.as_str(), None::<&str>);
          false
        }
      })
      .build()?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
