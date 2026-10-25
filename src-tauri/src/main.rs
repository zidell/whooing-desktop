// The locally built Windows app is a GUI too; debug logs are written by the log plugin.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
  app_lib::run();
}
