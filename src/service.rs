//! # Service — Platform-Specific Daemon Installation
//!
//! Installs and manages darkreach as a persistent background service.
//! Supports systemd (Linux) and launchd (macOS) for automatic restart,
//! log management, and auto-update on service restart.
//!
//! ## Usage
//!
//! ```bash
//! darkreach run --daemon      # Install and start as a background service
//! darkreach run --uninstall   # Stop and remove the service
//! ```
//!
//! ## Architecture
//!
//! On `--daemon`, the CLI generates a platform-specific service unit file
//! pointing at the current binary, enables it, and starts it. The service
//! is configured with `Restart=always` (systemd) or `KeepAlive=true`
//! (launchd) so the process restarts after updates or crashes.
//!
//! Environment variables `DARKREACH_AUTO_UPDATE=1` and
//! `DARKREACH_AUTO_UPDATE_APPLY=1` are set in the service unit so the
//! work loop automatically downloads, verifies, and applies binary updates.
//! After applying an update, the process exits and the service manager
//! restarts it with the new binary.

use anyhow::Result;

/// Install darkreach as a persistent background service.
///
/// - **Linux**: Generates `~/.config/systemd/user/darkreach-operator.service`
///   and enables it via `systemctl --user`.
/// - **macOS**: Generates `~/Library/LaunchAgents/ai.darkreach.operator.plist`
///   and loads it via `launchctl`.
pub fn install_service(database_url: &str) -> Result<()> {
    let binary = std::env::current_exe()?;
    let binary_path = binary
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Binary path is not valid UTF-8"))?;

    #[cfg(target_os = "linux")]
    {
        install_systemd_service(binary_path, database_url)?;
    }

    #[cfg(target_os = "macos")]
    {
        install_launchd_service(binary_path, database_url)?;
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binary_path, database_url);
        return Err(anyhow::anyhow!(
            "Daemon mode is only supported on Linux (systemd) and macOS (launchd)"
        ));
    }

    Ok(())
}

/// Uninstall the darkreach background service.
pub fn uninstall_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        uninstall_systemd_service()?;
    }

    #[cfg(target_os = "macos")]
    {
        uninstall_launchd_service()?;
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        return Err(anyhow::anyhow!(
            "Daemon mode is only supported on Linux (systemd) and macOS (launchd)"
        ));
    }

    Ok(())
}

/// Restart the service (used after binary update).
pub fn restart_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("systemctl")
            .args(["--user", "restart", "darkreach-operator"])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("systemctl restart failed"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let plist_path = launchd_plist_path()?;
        let _ = std::process::Command::new("launchctl")
            .args(["unload", plist_path.to_str().unwrap_or("")])
            .status();
        let status = std::process::Command::new("launchctl")
            .args(["load", plist_path.to_str().unwrap_or("")])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("launchctl load failed"));
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        return Err(anyhow::anyhow!("Restart not supported on this platform"));
    }

    Ok(())
}

/// Check if the current process was launched by a service manager.
///
/// - **Linux**: Checks for `INVOCATION_ID` (set by systemd).
/// - **macOS**: Checks for the launchd plist label in `LAUNCHED_BY` or
///   `XPC_SERVICE_NAME` environment.
pub fn is_managed_service() -> bool {
    // systemd sets INVOCATION_ID for every service invocation
    if std::env::var("INVOCATION_ID").is_ok() {
        return true;
    }
    // launchd sets __CF_USER_TEXT_ENCODING and XPC_SERVICE_NAME
    if let Ok(val) = std::env::var("XPC_SERVICE_NAME") {
        if val.contains("darkreach") {
            return true;
        }
    }
    // Also check our own env var that we set in the service unit
    std::env::var("DARKREACH_MANAGED_SERVICE").is_ok()
}

/// Return the platform-specific command to check service status.
pub fn status_command() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "systemctl --user status darkreach-operator"
    }
    #[cfg(target_os = "macos")]
    {
        "launchctl list ai.darkreach.operator"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        "echo 'Service management not supported on this platform'"
    }
}

/// Return the platform-specific command to view service logs.
pub fn logs_command() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "journalctl --user -u darkreach-operator -f"
    }
    #[cfg(target_os = "macos")]
    {
        "tail -f ~/.darkreach/darkreach.log"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        "echo 'Log viewing not supported on this platform'"
    }
}

// ── Linux (systemd) ──────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn systemd_unit_path() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot determine home directory"))?;
    let dir = std::path::PathBuf::from(home)
        .join(".config")
        .join("systemd")
        .join("user");
    Ok(dir.join("darkreach-operator.service"))
}

#[cfg(target_os = "linux")]
fn install_systemd_service(binary_path: &str, database_url: &str) -> Result<()> {
    let unit_path = systemd_unit_path()?;
    if let Some(parent) = unit_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let unit = format!(
        r#"[Unit]
Description=darkreach operator node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={binary} run
Restart=always
RestartSec=10
Environment=DATABASE_URL={db_url}
Environment=DARKREACH_AUTO_UPDATE=1
Environment=DARKREACH_AUTO_UPDATE_APPLY=1
Environment=DARKREACH_MANAGED_SERVICE=1
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
"#,
        binary = binary_path,
        db_url = database_url,
    );

    std::fs::write(&unit_path, unit)?;
    eprintln!("Wrote service unit to {}", unit_path.display());

    // Reload, enable, and start
    let status = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("systemctl daemon-reload failed"));
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "enable", "darkreach-operator"])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("systemctl enable failed"));
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "start", "darkreach-operator"])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("systemctl start failed"));
    }

    eprintln!("Service installed and started.");
    eprintln!("  Status: {}", status_command());
    eprintln!("  Logs:   {}", logs_command());

    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_systemd_service() -> Result<()> {
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "stop", "darkreach-operator"])
        .status();

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "disable", "darkreach-operator"])
        .status();

    let unit_path = systemd_unit_path()?;
    if unit_path.exists() {
        std::fs::remove_file(&unit_path)?;
        eprintln!("Removed {}", unit_path.display());
    }

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    eprintln!("Service uninstalled.");
    Ok(())
}

// ── macOS (launchd) ──────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn launchd_plist_path() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(std::path::PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("ai.darkreach.operator.plist"))
}

#[cfg(target_os = "macos")]
fn darkreach_log_path() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Cannot determine home directory"))?;
    let dir = std::path::PathBuf::from(home).join(".darkreach");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("darkreach.log"))
}

#[cfg(target_os = "macos")]
fn install_launchd_service(binary_path: &str, database_url: &str) -> Result<()> {
    let plist_path = launchd_plist_path()?;
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let log_path = darkreach_log_path()?;
    let log_str = log_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Log path is not valid UTF-8"))?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.darkreach.operator</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>run</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>DATABASE_URL</key>
        <string>{db_url}</string>
        <key>DARKREACH_AUTO_UPDATE</key>
        <string>1</string>
        <key>DARKREACH_AUTO_UPDATE_APPLY</key>
        <string>1</string>
        <key>DARKREACH_MANAGED_SERVICE</key>
        <string>1</string>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
</dict>
</plist>
"#,
        binary = binary_path,
        log = log_str,
        db_url = database_url,
    );

    std::fs::write(&plist_path, plist)?;
    eprintln!("Wrote plist to {}", plist_path.display());

    // Load the agent
    let status = std::process::Command::new("launchctl")
        .args(["load", plist_path.to_str().unwrap_or("")])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("launchctl load failed"));
    }

    eprintln!("Service installed and started.");
    eprintln!("  Status: {}", status_command());
    eprintln!("  Logs:   {}", logs_command());

    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_launchd_service() -> Result<()> {
    let plist_path = launchd_plist_path()?;

    if plist_path.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", plist_path.to_str().unwrap_or("")])
            .status();

        std::fs::remove_file(&plist_path)?;
        eprintln!("Removed {}", plist_path.display());
    }

    eprintln!("Service uninstalled.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_command_not_empty() {
        assert!(!status_command().is_empty());
    }

    #[test]
    fn logs_command_not_empty() {
        assert!(!logs_command().is_empty());
    }

    #[test]
    fn is_managed_service_returns_bool() {
        // In test context, should return false (not launched by systemd/launchd)
        let _ = is_managed_service();
    }
}
