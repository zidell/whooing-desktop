use tauri::UserAttentionType;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      set_notification_badge,
      notify_new_message
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
