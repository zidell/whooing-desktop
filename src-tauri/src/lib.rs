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

// whooing://<path>?<query> 형태의 딥링크(예: OAuth 콜백 핸드오프)를
// https://whooing.com/<path>?<query> 로 변환해 메인 윈도우를 이동시킨다.
fn handle_deep_link_url(app: &tauri::AppHandle, url: &tauri::Url) {
  let Some(window) = app.get_webview_window("main") else {
    return;
  };
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
  if let Ok(parsed) = target.parse() {
    let _ = window.navigate(parsed);
  }
  let _ = window.set_focus();
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
  let mut builder = tauri::Builder::default();

  // 싱글 인스턴스 플러그인은 반드시 제일 먼저 등록해야 한다.
  // Windows/Linux는 macOS와 달리 딥링크를 OS 이벤트가 아니라 "새 인스턴스 실행(argv)"로
  // 전달하는데, deep-link feature가 이 argv를 감지해 기존 인스턴스의
  // deep_link().on_open_url() 이벤트로 그대로 넘겨준다.
  #[cfg(desktop)]
  {
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
      }
    }));
  }

  builder
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

      // 앱이 딥링크로 "새로" 실행된 경우(콜드 스타트) — 이미 떠 있는 인스턴스에
      // 붙는 케이스는 single-instance 플러그인이 on_open_url로 넘겨주지만,
      // 최초 실행 시의 URL은 get_current()로 직접 확인해야 한다.
      let app_handle_for_current = app.handle().clone();
      if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
          handle_deep_link_url(&app_handle_for_current, &url);
        }
      }

      let deep_link_app_handle = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
          handle_deep_link_url(&deep_link_app_handle, &url);
        }
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
      .initialization_script(RELOAD_SHORTCUT_SCRIPT)
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
