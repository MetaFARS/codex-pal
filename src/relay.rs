use std::fmt;
use std::fs::{self, File};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::provider::ProviderProfile;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(10);
const HEALTH_POLL: Duration = Duration::from_millis(300);

#[derive(Debug, Clone)]
pub struct RelayRequest {
    pub relay_bin: String,
    pub provider: ProviderProfile,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RelayMetadata {
    pid: Option<u32>,
    port: u16,
    upstream: String,
    api_key_env: String,
}

impl RelayMetadata {
    fn new(pid: Option<u32>, request: &RelayRequest) -> Self {
        Self {
            pid,
            port: request.port,
            upstream: request.provider.upstream.clone(),
            api_key_env: request.provider.api_key_env.clone(),
        }
    }

    fn matches(&self, request: &RelayRequest) -> bool {
        self.port == request.port
            && self.upstream == request.provider.upstream
            && self.api_key_env == request.provider.api_key_env
    }
}

struct RelayReservation {
    metadata_path: PathBuf,
    pid_path: PathBuf,
    committed: bool,
}

impl RelayReservation {
    fn create(request: &RelayRequest, pid_path: PathBuf) -> Result<Self> {
        let metadata_path = metadata_file(request.port)?;
        write_metadata(&RelayMetadata::new(None, request))?;
        Ok(Self {
            metadata_path,
            pid_path,
            committed: false,
        })
    }

    fn record_pid(&self, pid: u32, request: &RelayRequest) -> Result<()> {
        fs::write(&self.pid_path, pid.to_string())
            .with_context(|| format!("writing relay pid {}", self.pid_path.display()))?;
        write_metadata(&RelayMetadata::new(Some(pid), request))
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for RelayReservation {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.pid_path);
            let _ = fs::remove_file(&self.metadata_path);
        }
    }
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
    let _lock = lock_port(request.port)?;
    let pid_file = pid_file(request.port)?;
    let log_file = log_file(request.port)?;
    let existing_pid = read_pid(&pid_file);
    let existing_metadata = read_metadata(request.port)?;
    if let Some(metadata) = &existing_metadata {
        if let Some(metadata_pid) = metadata.pid
            && Some(metadata_pid) != existing_pid
        {
            bail!(
                "relay state for port {} is inconsistent; stop the existing relay and retry",
                request.port
            );
        }
        if !metadata.matches(request) {
            bail!(
                "relay on port {} is configured for upstream {:?} with API key environment variable {:?}, not upstream {:?} with {:?}; use a different --port or stop the existing relay",
                request.port,
                metadata.upstream,
                metadata.api_key_env,
                request.provider.upstream,
                request.provider.api_key_env,
            );
        }
    }
    if port_in_use(request.port) {
        if existing_metadata.is_some() || existing_pid.is_some() {
            return Ok(RelayState::AlreadyRunning {
                pid: existing_metadata
                    .and_then(|metadata| metadata.pid)
                    .or(existing_pid),
                port: request.port,
            });
        }
        return Ok(RelayState::AlreadyRunning {
            pid: None,
            port: request.port,
        });
    }
    if existing_metadata.is_some() {
        bail!(
            "relay on port {} has an incomplete or stale startup reservation; run `codex-pal relay stop --port {}` and retry",
            request.port,
            request.port
        );
    }
    if existing_pid.is_some() {
        let _ = fs::remove_file(&pid_file);
    }

    let reservation = RelayReservation::create(request, pid_file)?;
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
    let pid = child.id();
    if let Err(error) = reservation.record_pid(pid, request) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    if let Err(error) = wait_healthy(&mut child, request.port, &log_file) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    reservation.commit();
    Ok(RelayState::Started {
        pid,
        port: request.port,
    })
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
    let _lock = lock_port(port)?;
    let path = pid_file(port)?;
    let Some(pid) = read_pid(&path) else {
        if port_in_use(port) && read_metadata(port)?.is_some() {
            bail!(
                "relay on port {port} has managed configuration state but no PID; refusing to clear it while the port is active"
            );
        }
        let _ = fs::remove_file(metadata_file(port)?);
        return Ok(format!("not running port={port}"));
    };
    terminate_pid(pid)?;
    if !wait_port_released(port) {
        bail!(
            "relay pid={pid} did not release port {port} within {:?}; preserving relay state",
            HEALTH_TIMEOUT
        );
    }
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(metadata_file(port)?);
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
    if Path::new(relay_bin).components().count() == 1 {
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(sibling) = sibling_executable(&current_exe, relay_bin)
            && sibling.is_file()
        {
            return Ok(sibling);
        }
        #[cfg(windows)]
        if let Some(dependency) = pipx_dependency_executable(relay_bin) {
            return Ok(dependency);
        }
    }

    which::which(relay_bin).with_context(|| {
        format!(
            "codex-relay binary {relay_bin:?} not found beside codex-pal or in PATH; install codex-relay or set --relay-bin"
        )
    })
}

#[cfg(windows)]
fn pipx_dependency_executable(relay_bin: &str) -> Option<PathBuf> {
    let home = pipx_home()?;
    pipx_dependency_executable_in_home(&home, relay_bin)
}

#[cfg(windows)]
fn pipx_dependency_executable_in_home(home: &Path, relay_bin: &str) -> Option<PathBuf> {
    let venvs = home.join("venvs");
    let mut environments = fs::read_dir(&venvs)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("codex-pal"))
        })
        .collect::<Vec<_>>();
    environments.sort();
    environments.into_iter().find_map(|environment| {
        let bin_dir = environment.join("Scripts");
        let candidate = sibling_executable(&bin_dir.join("codex-pal"), relay_bin)?;
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(windows)]
fn pipx_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("PIPX_HOME") {
        return Some(PathBuf::from(home));
    }
    let home = dirs::home_dir()?;
    let legacy = [home.join(".local").join("pipx"), home.join("pipx")];
    legacy
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| dirs::data_local_dir().map(|path| path.join("pipx")))
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

fn wait_healthy(child: &mut Child, port: u16, log_file: &Path) -> Result<()> {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            bail!(
                "codex-relay exited immediately with {status}; check {}",
                log_file.display()
            );
        }
        if port_in_use(port) {
            if let Some(status) = child.try_wait()? {
                bail!(
                    "codex-relay exited immediately with {status}; check {}",
                    log_file.display()
                );
            }
            return Ok(());
        }
        thread::sleep(HEALTH_POLL);
    }
    bail!(
        "codex-relay did not become healthy on port {} within {:?}; check {}",
        port,
        HEALTH_TIMEOUT,
        log_file.display()
    )
}

fn wait_port_released(port: u16) -> bool {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    while Instant::now() < deadline {
        if !port_in_use(port) {
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

fn lock_port(port: u16) -> Result<File> {
    let path = state_dir()?.join(format!("codex-relay-{port}.lock"));
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("opening relay lock {}", path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("locking relay port {port}"))?;
    Ok(file)
}

fn metadata_file(port: u16) -> Result<PathBuf> {
    Ok(state_dir()?.join(format!("codex-relay-{port}.json")))
}

fn read_metadata(port: u16) -> Result<Option<RelayMetadata>> {
    let path = metadata_file(port)?;
    if !path.exists() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("parsing {}", path.display()))
        .map(Some)
}

fn write_metadata(metadata: &RelayMetadata) -> Result<()> {
    let path = metadata_file(metadata.port)?;
    let contents = serde_json::to_vec_pretty(metadata)?;
    fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
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

    #[test]
    fn relay_metadata_matches_relay_configuration_not_profile_alias() {
        let request = RelayRequest {
            relay_bin: "codex-relay".to_string(),
            provider: ProviderProfile {
                name: "z".to_string(),
                upstream: "https://api.z.ai/api/paas/v4".to_string(),
                api_key_env: "ZAI_API_KEY".to_string(),
                api_key: None,
            },
            port: 4444,
        };
        let metadata = RelayMetadata::new(Some(42), &request);
        let alias_request = RelayRequest {
            provider: ProviderProfile {
                name: "zai".to_string(),
                ..request.provider.clone()
            },
            ..request.clone()
        };

        assert!(metadata.matches(&alias_request));
    }

    #[test]
    fn relay_metadata_rejects_a_different_upstream_or_key_environment() {
        let request = RelayRequest {
            relay_bin: "codex-relay".to_string(),
            provider: ProviderProfile {
                name: "deepseek".to_string(),
                upstream: "https://api.deepseek.com/v1".to_string(),
                api_key_env: "DEEPSEEK_API_KEY".to_string(),
                api_key: None,
            },
            port: 4444,
        };
        let metadata = RelayMetadata::new(Some(42), &request);
        let different_upstream = RelayRequest {
            provider: ProviderProfile {
                upstream: "https://llm.example.com/v1".to_string(),
                ..request.provider.clone()
            },
            ..request.clone()
        };
        let different_key_env = RelayRequest {
            provider: ProviderProfile {
                api_key_env: "OTHER_API_KEY".to_string(),
                ..request.provider.clone()
            },
            ..request.clone()
        };

        assert!(!metadata.matches(&different_upstream));
        assert!(!metadata.matches(&different_key_env));
    }

    #[cfg(windows)]
    #[test]
    fn locates_dependency_binary_in_copied_pipx_environment() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join("venvs").join("codex-pal").join("Scripts");
        fs::create_dir_all(&bin_dir).unwrap();
        let relay = sibling_executable(&bin_dir.join("codex-pal"), "codex-relay").unwrap();
        File::create(&relay).unwrap();

        let found = pipx_dependency_executable_in_home(temp.path(), "codex-relay");

        assert_eq!(found, Some(relay));
    }
}
