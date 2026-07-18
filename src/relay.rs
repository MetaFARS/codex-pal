use std::fmt;
use std::fs::{self, File};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::provider::ProviderProfile;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(10);
const HEALTH_POLL: Duration = Duration::from_millis(300);

#[derive(Debug, Clone)]
pub struct RelayRequest {
    pub relay_bin: String,
    pub provider: ProviderProfile,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayState {
    Started { pid: u32, port: u16 },
    AlreadyRunning { pid: Option<u32>, port: u16 },
}

impl fmt::Display for RelayState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayState::Started { pid, port } => write!(f, "started (pid={pid}, port={port})"),
            RelayState::AlreadyRunning {
                pid: Some(pid),
                port,
            } => write!(f, "already running (pid={pid}, port={port})"),
            RelayState::AlreadyRunning { pid: None, port } => {
                write!(f, "already running (external, port={port})")
            }
        }
    }
}

pub fn ensure_relay(request: &RelayRequest) -> Result<RelayState> {
    let pid_file = pid_file(request.port)?;
    let log_file = log_file(request.port)?;
    let existing_pid = read_pid(&pid_file);
    if existing_pid.is_some() && port_in_use(request.port) {
        return Ok(RelayState::AlreadyRunning {
            pid: existing_pid,
            port: request.port,
        });
    }
    if port_in_use(request.port) {
        return Ok(RelayState::AlreadyRunning {
            pid: None,
            port: request.port,
        });
    }
    let relay_bin = resolve_relay_bin(&request.relay_bin)?;

    let log = File::create(&log_file)
        .with_context(|| format!("creating relay log {}", log_file.display()))?;
    let log_err = log
        .try_clone()
        .with_context(|| format!("cloning relay log {}", log_file.display()))?;
    let mut cmd = Command::new(&relay_bin);
    cmd.arg("--port")
        .arg(request.port.to_string())
        .arg("--upstream")
        .arg(&request.provider.upstream)
        .env("CODEX_RELAY_PORT", request.port.to_string())
        .env("CODEX_RELAY_UPSTREAM", &request.provider.upstream)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));

    if let Some(api_key) = request.provider.api_key_value() {
        cmd.env(&request.provider.api_key_env, &api_key)
            .env("OPENAI_API_KEY", &api_key)
            .env("CODEX_RELAY_API_KEY", api_key);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {}", relay_bin.display()))?;
    fs::write(&pid_file, child.id().to_string())
        .with_context(|| format!("writing relay pid {}", pid_file.display()))?;

    if wait_healthy(request.port) {
        return Ok(RelayState::Started {
            pid: child.id(),
            port: request.port,
        });
    }
    if let Some(status) = child.try_wait()? {
        bail!(
            "codex-relay exited immediately with {status}; check {}",
            log_file.display()
        );
    }
    bail!(
        "codex-relay did not become healthy on port {} within {:?}; check {}",
        request.port,
        HEALTH_TIMEOUT,
        log_file.display()
    );
}

pub fn relay_status(port: u16) -> String {
    let pid = pid_file(port).ok().and_then(|path| read_pid(&path));
    let running = port_in_use(port);
    match (running, pid) {
        (true, Some(pid)) => format!("running managed pid={pid} port={port}"),
        (true, None) => format!("running external port={port}"),
        (false, Some(pid)) => format!("stale pid={pid} port={port}"),
        (false, None) => format!("not running port={port}"),
    }
}

pub fn stop_relay(port: u16) -> Result<String> {
    let path = pid_file(port)?;
    let Some(pid) = read_pid(&path) else {
        return Ok(format!("not running port={port}"));
    };
    terminate_pid(pid)?;
    let _ = fs::remove_file(path);
    Ok(format!("stopped pid={pid} port={port}"))
}

pub fn print_relay_config(relay_bin: &str, provider: &ProviderProfile) -> Result<()> {
    let relay_bin = resolve_relay_bin(relay_bin)?;
    let mut cmd = Command::new(&relay_bin);
    cmd.arg("--print-config")
        .arg("--upstream")
        .arg(&provider.upstream)
        .env("CODEX_RELAY_UPSTREAM", &provider.upstream);
    if let Some(api_key) = provider.api_key_value() {
        cmd.env(&provider.api_key_env, &api_key)
            .env("OPENAI_API_KEY", &api_key)
            .env("CODEX_RELAY_API_KEY", api_key);
    }
    let status = cmd.status()?;
    if !status.success() {
        bail!("codex-relay --print-config exited with {status}");
    }
    Ok(())
}

fn resolve_relay_bin(relay_bin: &str) -> Result<PathBuf> {
    if Path::new(relay_bin).components().count() == 1
        && let Ok(current_exe) = std::env::current_exe()
        && let Some(sibling) = sibling_executable(&current_exe, relay_bin)
        && sibling.is_file()
    {
        return Ok(sibling);
    }

    which::which(relay_bin).with_context(|| {
        format!(
            "codex-relay binary {relay_bin:?} not found beside codex-pal or in PATH; install codex-relay or set --relay-bin"
        )
    })
}

fn sibling_executable(current_exe: &Path, relay_bin: &str) -> Option<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    let file_name = if suffix.is_empty() || relay_bin.ends_with(suffix) {
        relay_bin.to_string()
    } else {
        format!("{relay_bin}{suffix}")
    };
    Some(current_exe.parent()?.join(file_name))
}

pub fn normalize_relay_base_url(raw: &str) -> Result<String> {
    let value = raw.trim().trim_end_matches('/');
    if value.is_empty() {
        bail!("relay URL must not be empty");
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        bail!("relay URL must start with http:// or https://");
    }
    if value.ends_with("/v1") {
        Ok(value.to_string())
    } else {
        Ok(format!("{value}/v1"))
    }
}

fn port_in_use(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

fn wait_healthy(port: u16) -> bool {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() < deadline {
        if port_in_use(port) {
            return true;
        }
        thread::sleep(HEALTH_POLL);
    }
    false
}

fn state_dir() -> Result<PathBuf> {
    let base = if let Some(raw) = std::env::var_os("CODEX_PAL_STATE_DIR") {
        PathBuf::from(raw)
    } else if let Some(dir) = dirs::state_dir() {
        dir.join("codex-pal")
    } else {
        dirs::home_dir()
            .context("cannot resolve home directory")?
            .join(".local")
            .join("state")
            .join("codex-pal")
    };
    fs::create_dir_all(&base).with_context(|| format!("creating state dir {}", base.display()))?;
    Ok(base)
}

fn pid_file(port: u16) -> Result<PathBuf> {
    Ok(state_dir()?.join(format!("codex-relay-{port}.pid")))
}

fn log_file(port: u16) -> Result<PathBuf> {
    Ok(state_dir()?.join(format!("codex-relay-{port}.log")))
}

fn read_pid(path: &PathBuf) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()
}

#[cfg(unix)]
fn terminate_pid(pid: u32) -> Result<()> {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .context("running kill -TERM")?;
    if !status.success() {
        bail!("kill -TERM {pid} exited with {status}");
    }
    Ok(())
}

#[cfg(windows)]
fn terminate_pid(pid: u32) -> Result<()> {
    let status = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/T")
        .arg("/F")
        .status()
        .context("running taskkill")?;
    if !status.success() {
        bail!("taskkill for pid {pid} exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_remote_relay_url_to_v1() {
        assert_eq!(
            normalize_relay_base_url("https://relay.example.com").unwrap(),
            "https://relay.example.com/v1"
        );
        assert_eq!(
            normalize_relay_base_url("https://relay.example.com/v1/").unwrap(),
            "https://relay.example.com/v1"
        );
        assert_eq!(
            normalize_relay_base_url("http://127.0.0.1:4444/base").unwrap(),
            "http://127.0.0.1:4444/base/v1"
        );
    }

    #[test]
    fn rejects_invalid_remote_relay_url() {
        assert!(normalize_relay_base_url("").is_err());
        assert!(normalize_relay_base_url("relay.example.com").is_err());
    }

    #[test]
    fn locates_dependency_binary_beside_pal_executable() {
        let current = Path::new("env")
            .join("bin")
            .join(format!("codex-pal{}", std::env::consts::EXE_SUFFIX));
        assert_eq!(
            sibling_executable(&current, "codex-relay"),
            Some(
                Path::new("env")
                    .join("bin")
                    .join(format!("codex-relay{}", std::env::consts::EXE_SUFFIX))
            )
        );
    }
}
