use std::process::{Command, ExitStatus};

use anyhow::{Result, bail};

use crate::provider::ProviderProfile;
use crate::toml_value;

#[derive(Debug, Clone)]
pub struct CodexLaunch {
    pub codex_bin: String,
    pub provider: ProviderProfile,
    pub port: u16,
    pub model: Option<String>,
    pub ask: bool,
    pub no_sandbox: bool,
    pub context_window: u32,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCommand {
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn build_codex_command(launch: &CodexLaunch) -> Result<CodexCommand> {
    if which::which(&launch.codex_bin).is_err() {
        bail!(
            "codex binary {:?} not found in PATH; install Codex or set --codex-bin",
            launch.codex_bin
        );
    }

    let mut argv = vec![launch.codex_bin.clone()];
    let mut env = Vec::new();

    if launch.provider.needs_relay() {
        let key_value = launch.provider.api_key_value().unwrap_or_default();
        if !key_value.is_empty() {
            env.push((launch.provider.api_key_env.clone(), key_value.clone()));
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
            &toml_value::string(&format!("http://127.0.0.1:{}/v1", launch.port)),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.wire_api",
            &toml_value::string("responses"),
        );
        push_config(
            &mut argv,
            "model_providers.codex-pal.env_key",
            &toml_value::string(&launch.provider.api_key_env),
        );
    }

    if let Some(model) = &launch.model {
        argv.push("-m".to_string());
        argv.push(model.clone());
        push_model_properties(&mut argv, model, launch.context_window);
    }

    if launch.no_sandbox {
        if launch.ask {
            eprintln!("note: --ask ignored because --no-sandbox bypasses approvals too");
        }
        argv.push("--dangerously-bypass-approvals-and-sandbox".to_string());
    } else if !launch.ask {
        push_config(&mut argv, "approval_policy", &toml_value::string("never"));
        argv.push("-s".to_string());
        argv.push("workspace-write".to_string());
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

fn push_config(argv: &mut Vec<String>, key: &str, value: &str) {
    argv.push("-c".to_string());
    argv.push(format!("{key}={value}"));
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
            model: Some("deepseek-chat".to_string()),
            ask: false,
            no_sandbox: false,
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
                .contains(&"model_properties.\"deepseek-chat\".context_window=64000".to_string())
        );
        assert!(
            command
                .env
                .contains(&("DEEPSEEK_API_KEY".to_string(), "dummy".to_string()))
        );
    }
}
