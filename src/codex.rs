use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::provider::{ProviderModel, ProviderProfile, provider_models};
use crate::toml_value;

#[derive(Debug, Clone)]
pub struct CodexLaunch {
    pub codex_bin: String,
    pub provider: ProviderProfile,
    pub port: u16,
    pub relay_base_url: Option<String>,
    pub model: Option<String>,
    pub model_catalog_json: Option<String>,
    pub approval: ApprovalPolicy,
    pub sandbox: SandboxMode,
    pub context_window: u32,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalPolicy {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

impl ApprovalPolicy {
    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::OnRequest => "on-request",
            Self::OnFailure => "on-failure",
            Self::Untrusted => "untrusted",
        }
    }
}

impl std::str::FromStr for ApprovalPolicy {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "never" => Ok(Self::Never),
            "on-request" => Ok(Self::OnRequest),
            "on-failure" => Ok(Self::OnFailure),
            "untrusted" => Ok(Self::Untrusted),
            other => bail!("unknown approval policy {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    pub fn as_cli_value(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
        }
    }

    pub fn bypasses_sandbox(self) -> bool {
        self == Self::DangerFullAccess
    }
}

impl std::str::FromStr for SandboxMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "read-only" => Ok(Self::ReadOnly),
            "workspace-write" => Ok(Self::WorkspaceWrite),
            "danger-full-access" => Ok(Self::DangerFullAccess),
            other => bail!("unknown sandbox mode {other:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCommand {
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn build_codex_command(launch: &CodexLaunch) -> Result<CodexCommand> {
    let codex_bin = resolve_codex_bin(&launch.codex_bin)?;
    let mut argv = vec![codex_bin.display().to_string()];
    let mut env = Vec::new();

    if launch.provider.needs_relay() || launch.relay_base_url.is_some() {
        let configured_key = launch
            .provider
            .api_key_value()
            .filter(|key| !key.is_empty());
        let uses_remote_relay = launch.relay_base_url.is_some();
        let codex_env_key = if configured_key.is_some() || !uses_remote_relay {
            launch.provider.api_key_env.clone()
        } else {
            "CODEX_PAL_RELAY_API_KEY".to_string()
        };
        let key_value = configured_key
            .or_else(|| uses_remote_relay.then(|| "codex-pal-remote-relay".to_string()));
        if let Some(key_value) = key_value {
            env.push((codex_env_key.clone(), key_value.clone()));
            env.push(("OPENAI_API_KEY".to_string(), key_value));
        }

        push_config(
            &mut argv,
            "model_provider",
            &toml_value::string("codex-pal"),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.name",
            &toml_value::string(&format!("codex-pal ({})", launch.provider.name)),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.base_url",
            &toml_value::string(&relay_base_url(launch)),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.wire_api",
            &toml_value::string("responses"),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.env_key",
            &toml_value::string(&codex_env_key),
        );
        if let Some(path) = &launch.model_catalog_json {
            push_config(&mut argv, "model_catalog_json", &toml_value::string(path));
        }
    }

    if let Some(model) = &launch.model {
        argv.push("-m".to_string());
        argv.push(model.clone());
        push_model_properties(&mut argv, model, launch.context_window);
    }

    if launch.sandbox.bypasses_sandbox() {
        argv.push("--dangerously-bypass-approvals-and-sandbox".to_string());
    } else {
        push_config(
            &mut argv,
            "approval_policy",
            &toml_value::string(launch.approval.as_config_value()),
        );
        argv.push("-s".to_string());
        argv.push(launch.sandbox.as_cli_value().to_string());
    }

    argv.extend(launch.extra_args.clone());
    Ok(CodexCommand { argv, env })
}

pub fn exec_codex(command: CodexCommand) -> Result<ExitStatus> {
    let mut cmd = Command::new(&command.argv[0]);
    cmd.args(&command.argv[1..]);
    for (key, value) in command.env {
        cmd.env(key, value);
    }
    Ok(cmd.status()?)
}

pub fn write_provider_model_catalog(
    codex_bin: &str,
    provider: &ProviderProfile,
    port: u16,
    selected_model: Option<&str>,
) -> Result<Option<PathBuf>> {
    let mut models = provider_models(&provider.name)
        .iter()
        .map(|model| CatalogModel {
            slug: model.slug.to_string(),
            display_name: model.display_name.to_string(),
            description: model.description.to_string(),
            context_window: model.context_window,
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        return Ok(None);
    }
    match selected_model {
        Some(selected_model) if !models.iter().any(|model| model.slug == selected_model) => {
            models.push(CatalogModel {
                slug: selected_model.to_string(),
                display_name: selected_model.to_string(),
                description: format!("Custom {} model selected for this launch.", provider.name),
                context_window: 128_000,
            });
        }
        _ => {}
    }

    let template = bundled_model_template(codex_bin)?;
    let entries = models
        .iter()
        .enumerate()
        .map(|(priority, model)| catalog_entry_from_template(&template, model, priority))
        .collect::<Result<Vec<_>>>()?;
    let catalog = serde_json::json!({ "models": entries });
    let path = model_catalog_file(&provider.name, port)?;
    fs::write(&path, serde_json::to_vec_pretty(&catalog)?)
        .with_context(|| format!("writing model catalog {}", path.display()))?;
    Ok(Some(path))
}

fn push_config(argv: &mut Vec<String>, key: &str, value: &str) {
    argv.push("-c".to_string());
    argv.push(format!("{key}={value}"));
}

fn relay_base_url(launch: &CodexLaunch) -> String {
    launch
        .relay_base_url
        .clone()
        .unwrap_or_else(|| format!("http://127.0.0.1:{}/v1", launch.port))
}

#[derive(Debug)]
struct CatalogModel {
    slug: String,
    display_name: String,
    description: String,
    context_window: u32,
}

impl From<&ProviderModel> for CatalogModel {
    fn from(model: &ProviderModel) -> Self {
        Self {
            slug: model.slug.to_string(),
            display_name: model.display_name.to_string(),
            description: model.description.to_string(),
            context_window: model.context_window,
        }
    }
}

fn bundled_model_template(codex_bin: &str) -> Result<Value> {
    #[cfg(windows)]
    let resolved_codex_bin = resolve_codex_bin(codex_bin)?;
    #[cfg(not(windows))]
    let resolved_codex_bin = PathBuf::from(codex_bin);
    let output = Command::new(&resolved_codex_bin)
        .args(["debug", "models", "--bundled"])
        .output()
        .with_context(|| format!("reading bundled model catalog from {codex_bin:?}"))?;
    if !output.status.success() {
        bail!("codex debug models --bundled exited with {}", output.status);
    }
    let catalog: Value = serde_json::from_slice(&output.stdout)
        .context("parsing bundled Codex model catalog JSON")?;
    catalog
        .get("models")
        .and_then(Value::as_array)
        .and_then(|models| models.first())
        .cloned()
        .context("bundled Codex model catalog did not contain any models")
}

#[cfg(windows)]
fn resolve_codex_bin(codex_bin: &str) -> Result<PathBuf> {
    which::which(codex_bin).with_context(|| {
        format!("codex binary {codex_bin:?} not found in PATH; install Codex or set --codex-bin")
    })
}

#[cfg(not(windows))]
fn resolve_codex_bin(codex_bin: &str) -> Result<PathBuf> {
    which::which(codex_bin).with_context(|| {
        format!("codex binary {codex_bin:?} not found in PATH; install Codex or set --codex-bin")
    })?;
    Ok(PathBuf::from(codex_bin))
}

fn catalog_entry_from_template(
    template: &Value,
    model: &CatalogModel,
    priority: usize,
) -> Result<Value> {
    let mut entry = template.clone();
    let Some(object) = entry.as_object_mut() else {
        bail!("bundled Codex model template was not a JSON object");
    };
    object.insert("slug".to_string(), Value::String(model.slug.clone()));
    object.insert(
        "display_name".to_string(),
        Value::String(model.display_name.clone()),
    );
    object.insert(
        "description".to_string(),
        Value::String(model.description.clone()),
    );
    object.insert(
        "priority".to_string(),
        Value::Number(serde_json::Number::from(priority)),
    );
    object.insert("visibility".to_string(), Value::String("list".to_string()));
    object.insert("supported_in_api".to_string(), Value::Bool(true));
    object.insert(
        "context_window".to_string(),
        Value::Number(serde_json::Number::from(model.context_window)),
    );
    object.insert(
        "max_context_window".to_string(),
        Value::Number(serde_json::Number::from(model.context_window)),
    );
    object.insert(
        "input_modalities".to_string(),
        Value::Array(vec![Value::String("text".to_string())]),
    );
    object.insert("supports_search_tool".to_string(), Value::Bool(false));
    object.insert(
        "supports_image_detail_original".to_string(),
        Value::Bool(false),
    );
    object.insert(
        "supports_reasoning_summaries".to_string(),
        Value::Bool(false),
    );
    Ok(entry)
}

fn model_catalog_file(provider: &str, port: u16) -> Result<PathBuf> {
    let provider = provider
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    Ok(state_dir()?.join(format!("models-{provider}-{port}.json")))
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

fn push_model_properties(argv: &mut Vec<String>, model: &str, context_window: u32) {
    let key_prefix = format!("model_properties.{}.{}", quoted_key(model), "{}");
    push_config(
        argv,
        &key_prefix.replace("{}", "context_window"),
        &context_window.to_string(),
    );
    push_config(
        argv,
        &key_prefix.replace("{}", "max_context_window"),
        &context_window.to_string(),
    );
    push_config(
        argv,
        &key_prefix.replace("{}", "supports_parallel_tool_calls"),
        "true",
    );
    push_config(
        argv,
        &key_prefix.replace("{}", "supports_reasoning_summaries"),
        "false",
    );
    push_config(
        argv,
        &key_prefix.replace("{}", "input_modalities"),
        &toml_value::string_array(&["text"]),
    );
}

fn quoted_key(value: &str) -> String {
    toml_value::string(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn uses_resolved_codex_executable() {
        let executable = std::env::current_exe().unwrap();
        let launch = CodexLaunch {
            codex_bin: executable.display().to_string(),
            provider: ProviderProfile {
                name: "openai".to_string(),
                upstream: "https://api.openai.com/v1".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
                api_key: None,
            },
            port: 4444,
            relay_base_url: None,
            model: None,
            model_catalog_json: None,
            approval: ApprovalPolicy::Never,
            sandbox: SandboxMode::WorkspaceWrite,
            context_window: 128_000,
            extra_args: Vec::new(),
        };

        let command = build_codex_command(&launch).unwrap();

        assert_eq!(PathBuf::from(&command.argv[0]), executable);
    }

    #[cfg(windows)]
    #[test]
    fn resolves_cmd_executable_from_extensionless_name() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("codex.cmd");
        fs::write(&script, "@exit /b 0\r\n").unwrap();

        let resolved = resolve_codex_bin(&temp.path().join("codex").display().to_string()).unwrap();

        assert_eq!(resolved, script);
    }

    #[test]
    fn builds_relay_config_args() {
        let launch = CodexLaunch {
            codex_bin: "true".to_string(),
            provider: ProviderProfile {
                name: "deepseek".to_string(),
                upstream: "https://api.deepseek.com/v1".to_string(),
                api_key_env: "DEEPSEEK_API_KEY".to_string(),
                api_key: Some("dummy".to_string()),
            },
            port: 4567,
            relay_base_url: None,
            model: Some("deepseek-v4-pro".to_string()),
            model_catalog_json: None,
            approval: ApprovalPolicy::Never,
            sandbox: SandboxMode::WorkspaceWrite,
            context_window: 64_000,
            extra_args: vec!["--oss".to_string()],
        };
        let command = build_codex_command(&launch).unwrap();
        assert!(
            command
                .argv
                .contains(&"model_provider=\"codex-pal\"".to_string())
        );
        assert!(command.argv.contains(
            &"model_providers.codex-pal.base_url=\"http://127.0.0.1:4567/v1\"".to_string()
        ));
        assert!(
            command
                .argv
                .contains(&"approval_policy=\"never\"".to_string())
        );
        assert!(command.argv.contains(&"-s".to_string()));
        assert!(command.argv.contains(&"workspace-write".to_string()));
        assert!(
            command
                .argv
                .contains(&"model_properties.\"deepseek-v4-pro\".context_window=64000".to_string())
        );
        assert!(
            command
                .env
                .contains(&("DEEPSEEK_API_KEY".to_string(), "dummy".to_string()))
        );
    }

    #[test]
    fn builds_remote_relay_config_args_with_dummy_key() {
        let launch = CodexLaunch {
            codex_bin: "true".to_string(),
            provider: ProviderProfile {
                name: "deepseek".to_string(),
                upstream: "https://api.deepseek.com/v1".to_string(),
                api_key_env: "CODEX_PAL_TEST_MISSING_API_KEY".to_string(),
                api_key: None,
            },
            port: 4567,
            relay_base_url: Some("https://relay.example.com/v1".to_string()),
            model: Some("deepseek-v4-pro".to_string()),
            model_catalog_json: None,
            approval: ApprovalPolicy::Never,
            sandbox: SandboxMode::WorkspaceWrite,
            context_window: 64_000,
            extra_args: Vec::new(),
        };
        let command = build_codex_command(&launch).unwrap();
        assert!(command.argv.contains(
            &"model_providers.codex-pal.base_url=\"https://relay.example.com/v1\"".to_string()
        ));
        assert!(command.argv.contains(
            &"model_providers.codex-pal.env_key=\"CODEX_PAL_RELAY_API_KEY\"".to_string()
        ));
        assert!(command.env.contains(&(
            "CODEX_PAL_RELAY_API_KEY".to_string(),
            "codex-pal-remote-relay".to_string()
        )));
    }
}
