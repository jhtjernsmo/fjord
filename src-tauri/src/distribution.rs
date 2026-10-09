//! How this copy of Fjord was installed. The Microsoft Store version updates
//! through the Store, so the in-app updater stays out of its way.

use std::path::Path;

/// Store (MSIX) apps always run from the protected `WindowsApps` folder.
pub fn is_store_path(exe: &Path) -> bool {
    exe.components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("WindowsApps"))
}

/// True when running as the Microsoft Store (MSIX) package.
#[tauri::command]
pub fn installed_from_store() -> bool {
    cfg!(windows)
        && std::env::current_exe()
            .map(|exe| is_store_path(&exe))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_store_install_folder() {
        let store =
            Path::new("C:/Program Files/WindowsApps/Fjord_0.2.13.0_x64__8wekyb3d8bbwe/fjord.exe");
        assert!(is_store_path(store));
        assert!(is_store_path(Path::new(
            "C:/Program Files/windowsapps/x/fjord.exe"
        )));
    }

    #[test]
    fn regular_installs_are_not_the_store() {
        for exe in [
            "C:/Users/jonas/AppData/Local/Fjord/fjord.exe",
            "C:/Program Files/Fjord/fjord.exe",
            "C:/Users/jonas/WindowsAppsBackup/fjord.exe",
            "/usr/bin/fjord",
        ] {
            assert!(!is_store_path(Path::new(exe)), "{exe}");
        }
    }
}
