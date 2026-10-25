//! Register the explicitly enabled local preview as a separate default-app
//! candidate. Windows Settings chooses the default; never write UserChoice.
pub fn register(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  if !app
    .config()
    .plugins
    .0
    .get("whooing")
    .and_then(|settings| settings.get("localPreview"))
    .and_then(|value| value.as_bool())
    .unwrap_or(false)
  {
    return Ok(());
  }
  use windows_registry::CURRENT_USER;
  let executable = std::env::current_exe()?.display().to_string();
  let executable = executable.strip_prefix(r"\\?\").unwrap_or(&executable);
  let prog_id = "WhooingDesktopPreview.Url.whooing";
  let class = format!(r"Software\Classes\{prog_id}");
  let name = "후잉 (로컬 테스트)";
  let key = CURRENT_USER.create(&class)?;
  key.set_string("", "URL:Whooing local preview")?;
  key.set_string("URL Protocol", "")?;
  CURRENT_USER
    .create(format!(r"{class}\Application"))?
    .set_string("ApplicationName", name)?;
  CURRENT_USER
    .create(format!(r"{class}\DefaultIcon"))?
    .set_string("", format!("\"{executable}\",0"))?;
  CURRENT_USER
    .create(format!(r"{class}\shell\open\command"))?
    .set_string("", format!("\"{executable}\" \"%1\""))?;
  let capabilities = r"Software\WhooingDesktopPreview\Capabilities";
  let key = CURRENT_USER.create(capabilities)?;
  key.set_string("ApplicationName", name)?;
  key.set_string("ApplicationDescription", "Whooing local desktop preview")?;
  key.set_string("ApplicationIcon", format!("\"{executable}\",0"))?;
  CURRENT_USER
    .create(format!(r"{capabilities}\URLAssociations"))?
    .set_string("whooing", prog_id)?;
  CURRENT_USER
    .create(r"Software\RegisteredApplications")?
    .set_string(name, capabilities)?;
  CURRENT_USER
    .create(r"Software\Classes\whooing\OpenWithProgids")?
    .set_string(prog_id, "")?;
  Ok(())
}
