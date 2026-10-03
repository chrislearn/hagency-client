//! ADR-189: `hagency start` helpers and per-user service registration.
//!
//! The service runs as the user who installs it (a macOS LaunchAgent or a
//! `systemd --user` unit), so it sees that user's coding-agent sign-in and
//! `PATH`. It runs `hagency start --no-open`; the install command prints and
//! opens the console sign-in link once the service answers.
use std::{
    io::IsTerminal,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(clap::Subcommand)]
pub enum Command {
    /// Register and start the per-user service, then open the console.
    Install {
        #[arg(long)]
        state_dir: Option<PathBuf>,
        #[arg(long, default_value = "127.0.0.1:13300")]
        listen: SocketAddr,
        /// Do not open the console in the browser.
        #[arg(long)]
        no_open: bool,
    },
    /// Stop and remove the per-user service. The state directory is kept.
    Uninstall,
}

const LABEL: &str = "io.hagency";

/// `~/Library/Application Support/Hagency` (macOS) or
/// `$XDG_DATA_HOME/hagency`, else `~/.local/share/hagency` (Linux).
pub fn default_state_dir() -> Result<PathBuf, String> {
    let home = home()?;
    if cfg!(target_os = "macos") {
        return Ok(home.join("Library/Application Support/Hagency"));
    }
    Ok(match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("hagency"),
        _ => home.join(".local/share/hagency"),
    })
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_owned())
}

/// Wait until the service answers, then print its console sign-in link and,
/// when asked and this is an interactive terminal, open it in the browser.
pub async fn announce(state: PathBuf, address: SocketAddr, open: bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        // The sign-in link grants console access: it goes to an interactive
        // terminal only, never into a service log.
        if !std::io::stdout().is_terminal() {
            if crate::console::client::reachable(address).await {
                println!(
                    "Hagency is ready. Open the console with: hagency console-access --state-dir {} --listen {address}",
                    state.display()
                );
                return;
            }
            if Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }
        match crate::console::client::access(&state, address).await {
            Ok(link) => {
                println!("Hagency console: {link}");
                if open {
                    open_browser(&link);
                }
                return;
            }
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(_) => {
                eprintln!(
                    "Hagency did not answer within 60 s; run `hagency console-access --state-dir {} --listen {address}` once it is up",
                    state.display()
                );
                return;
            }
        }
    }
}

fn open_browser(link: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener)
        .arg(link)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn run(command: Command) -> Result<String, String> {
    match command {
        Command::Install {
            state_dir,
            listen,
            no_open,
        } => {
            let state = match state_dir {
                Some(dir) => dir,
                None => default_state_dir()?,
            };
            if !listen.ip().is_loopback() {
                return Err("--listen must be a loopback address".into());
            }
            let exe = std::env::current_exe()
                .and_then(|p| p.canonicalize())
                .map_err(|e| format!("this binary's path: {e}"))?;
            let path = std::env::var("PATH").unwrap_or_default();
            if cfg!(target_os = "macos") {
                install_launchd(&exe, &state, listen, &path)?;
            } else {
                install_systemd(&exe, &state, listen, &path)?;
            }
            let runtime = tokio::runtime::Handle::try_current();
            let announce = announce(state.clone(), listen, !no_open);
            match runtime {
                Ok(handle) => tokio::task::block_in_place(|| handle.block_on(announce)),
                Err(_) => tokio::runtime::Runtime::new()
                    .map_err(|e| e.to_string())?
                    .block_on(announce),
            }
            Ok(format!(
                "Hagency is installed as a service (state: {}).",
                state.display()
            ))
        }
        Command::Uninstall => {
            if cfg!(target_os = "macos") {
                uninstall_launchd()
            } else {
                uninstall_systemd()
            }
        }
    }
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn launchd_plist(exe: &Path, state: &Path, listen: SocketAddr, path: &str, log: &Path) -> String {
    let args = [
        exe.to_string_lossy().into_owned(),
        "start".into(),
        "--state-dir".into(),
        state.to_string_lossy().into_owned(),
        "--listen".into(),
        listen.to_string(),
        "--no-open".into(),
    ];
    let args: String = args
        .iter()
        .map(|a| format!("    <string>{}</string>\n", xml(a)))
        .collect();
    let log = xml(&log.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
{args}  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>{path}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#,
        path = xml(path),
    )
}

fn launchd_path() -> Result<PathBuf, String> {
    Ok(home()?.join(format!("Library/LaunchAgents/{LABEL}.plist")))
}

fn launchctl(args: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("launchctl")
        .args(args)
        .output()
        .map_err(|e| format!("launchctl: {e}"))
}

fn uid() -> String {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn install_launchd(exe: &Path, state: &Path, listen: SocketAddr, path: &str) -> Result<(), String> {
    let logs = home()?.join("Library/Logs/Hagency");
    std::fs::create_dir_all(&logs).map_err(|e| format!("{}: {e}", logs.display()))?;
    let plist = launchd_path()?;
    if let Some(parent) = plist.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let domain = format!("gui/{}", uid());
    // Replacing an installed agent: stop the old one first, or bootstrap fails.
    let _ = launchctl(&["bootout", &format!("{domain}/{LABEL}")]);
    std::fs::write(
        &plist,
        launchd_plist(exe, state, listen, path, &logs.join("hagency.log")),
    )
    .map_err(|e| format!("{}: {e}", plist.display()))?;
    let out = launchctl(&["bootstrap", &domain, &plist.to_string_lossy()])?;
    if !out.status.success() {
        return Err(format!(
            "launchctl bootstrap failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

fn uninstall_launchd() -> Result<String, String> {
    let plist = launchd_path()?;
    let _ = launchctl(&["bootout", &format!("gui/{}/{LABEL}", uid())]);
    if plist.exists() {
        std::fs::remove_file(&plist).map_err(|e| format!("{}: {e}", plist.display()))?;
    }
    Ok("Hagency service removed; its state directory was kept.".into())
}

fn systemd_unit(exe: &Path, state: &Path, listen: SocketAddr, path: &str) -> String {
    let quote = |p: &str| format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\""));
    format!(
        "[Unit]\nDescription=Hagency\nAfter=network-online.target\n\n[Service]\nExecStart={} start --state-dir {} --listen {listen} --no-open\nEnvironment={}\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=default.target\n",
        quote(&exe.to_string_lossy()),
        quote(&state.to_string_lossy()),
        quote(&format!("PATH={path}")),
    )
}

fn systemd_path() -> Result<PathBuf, String> {
    let config = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home()?.join(".config"),
    };
    Ok(config.join("systemd/user/hagency.service"))
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let out = std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl --user: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "systemctl --user {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

fn install_systemd(exe: &Path, state: &Path, listen: SocketAddr, path: &str) -> Result<(), String> {
    let unit = systemd_path()?;
    if let Some(parent) = unit.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&unit, systemd_unit(exe, state, listen, path))
        .map_err(|e| format!("{}: {e}", unit.display()))?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", "hagency.service"])?;
    // A user service stops at logout unless lingering is on.
    eprintln!(
        "Note: to keep Hagency running after you log out, run `loginctl enable-linger $USER` once."
    );
    Ok(())
}

fn uninstall_systemd() -> Result<String, String> {
    let _ = systemctl(&["disable", "--now", "hagency.service"]);
    let unit = systemd_path()?;
    if unit.exists() {
        std::fs::remove_file(&unit).map_err(|e| format!("{}: {e}", unit.display()))?;
    }
    let _ = systemctl(&["daemon-reload"]);
    Ok("Hagency service removed; its state directory was kept.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchd_plist_runs_start_without_opening_and_escapes_paths() {
        let plist = launchd_plist(
            Path::new("/opt/h & co/hagency"),
            Path::new("/Users/a/Library/Application Support/Hagency"),
            "127.0.0.1:13300".parse().unwrap(),
            "/usr/bin:/bin",
            Path::new("/Users/a/Library/Logs/Hagency/hagency.log"),
        );
        assert!(plist.contains("<string>/opt/h &amp; co/hagency</string>"));
        assert!(plist.contains("<string>start</string>"));
        assert!(plist.contains("<string>--no-open</string>"));
        assert!(plist.contains("<string>/Users/a/Library/Application Support/Hagency</string>"));
        assert!(plist.contains("<key>PATH</key>"));
        assert!(!plist.contains("--agent-driver"));
    }

    #[test]
    fn systemd_unit_quotes_paths_and_carries_path() {
        let unit = systemd_unit(
            Path::new("/opt/hagency/hagency"),
            Path::new("/home/a/.local/share/hagency"),
            "127.0.0.1:13300".parse().unwrap(),
            "/usr/bin:/bin",
        );
        assert!(unit.contains(
            "ExecStart=\"/opt/hagency/hagency\" start --state-dir \"/home/a/.local/share/hagency\" --listen 127.0.0.1:13300 --no-open"
        ));
        assert!(unit.contains("Environment=\"PATH=/usr/bin:/bin\""));
        assert!(unit.contains("WantedBy=default.target"));
    }
}
