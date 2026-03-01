use std::process::Command;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_API: &str =
    "https://api.github.com/repos/christestet/owui-lint/releases/latest";
const INSTALLER_SH: &str =
    "https://github.com/christestet/owui-lint/releases/latest/download/owui-lint-installer.sh";
const INSTALLER_PS1: &str =
    "https://github.com/christestet/owui-lint/releases/latest/download/owui-lint-installer.ps1";

/// Returns the latest release version (without leading 'v') if it is newer
/// than the currently running binary, or `None` if we are up-to-date (or on
/// any network / parse error).
pub fn check_latest_version() -> Option<String> {
    let body: String = ureq::get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "owui-lint")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;

    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    let latest = tag.strip_prefix('v').unwrap_or(tag);

    if latest != CURRENT_VERSION {
        Some(latest.to_string())
    } else {
        None
    }
}

/// `owui-lint update` entry-point.  Returns an exit code.
pub fn run_update() -> i32 {
    println!("owui-lint v{CURRENT_VERSION}");
    println!("Checking for updates...");

    let latest = match check_latest_version() {
        Some(v) => v,
        None => {
            println!("owui-lint is up to date (v{CURRENT_VERSION})");
            return 0;
        }
    };

    println!("New version available: v{latest} (current: v{CURRENT_VERSION})");
    println!("Downloading and installing...");

    let status = if cfg!(windows) {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{INSTALLER_PS1}' -OutFile $env:TEMP\\owui-lint-installer.ps1; \
                     & $env:TEMP\\owui-lint-installer.ps1"
                ),
            ])
            .status()
    } else {
        Command::new("sh")
            .args([
                "-c",
                &format!(
                    "curl --proto '=https' --tlsv1.2 -LsSf '{INSTALLER_SH}' | sh"
                ),
            ])
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            println!("Updated to v{latest} successfully!");
            0
        }
        Ok(s) => {
            eprintln!(
                "Installer exited with code {}",
                s.code().unwrap_or(-1)
            );
            1
        }
        Err(e) => {
            eprintln!("Failed to run installer: {e}");
            1
        }
    }
}
