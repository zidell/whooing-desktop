fn main() {
  tauri_build::try_build(
    tauri_build::Attributes::new().app_manifest(
      tauri_build::AppManifest::new()
        .commands(&["set_notification_badge", "notify_new_message"]),
    ),
  )
  .expect("failed to run tauri-build");
}
