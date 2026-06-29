use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::codex::{CodexLaunch, build_codex_command, exec_codex};
use crate::provider::ProviderProfile;
use crate::relay::{RelayRequest, ensure_relay, relay_status, stop_relay};

#[derive(Debug, Parser)]
#[command(
    name = "codex-pal",
    version,
    about = "Launch Codex through codex-relay for OpenAI-compatible providers",
    long_about = "codex-pal starts or reuses a codex-relay sidecar, injects Codex config overrides, and then hands the terminal to codex."
)]
pub struct Cli {
    #[command(flatten)]
    pub launch: LaunchArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print relay status for the selected port.
    Status(StatusArgs),
    /// Stop the managed relay for the selected port.
    Stop(StatusArgs),
    /// Print a Codex config.toml snippet by delegating to codex-relay --print-config.
    Config(ConfigArgs),
}

#[derive(Debug, Args, Clone)]
pub struct StatusArgs {
    /// Relay port to inspect.
    #[arg(long, env = "CODEX_PAL_PORT", default_value_t = 4444)]
    pub port: u16,
}

#[derive(Debug, Args, Clone)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub provider: ProviderArgs,

    /// codex-relay executable.
    #[arg(long, env = "CODEX_PAL_RELAY_BIN", default_value = "codex-relay")]
    pub relay_bin: String,
}

#[derive(Debug, Args, Clone)]
pub struct LaunchArgs {
    #[command(flatten)]
    pub provider: ProviderArgs,

    /// Model passed to Codex with -m.
    #[arg(short = 'm', long, env = "CODEX_PAL_MODEL")]
    pub model: Option<String>,

    /// Codex executable.
    #[arg(long, env = "CODEX_PAL_CODEX_BIN", default_value = "codex")]
    pub codex_bin: String,

    /// codex-relay executable.
    #[arg(long, env = "CODEX_PAL_RELAY_BIN", default_value = "codex-relay")]
    pub relay_bin: String,

    /// Relay port.
    #[arg(long, env = "CODEX_PAL_PORT", default_value_t = 4444)]
    pub port: u16,

    /// Leave Codex approval prompts at their configured default.
    #[arg(long, env = "CODEX_PAL_ASK", default_value_t = false)]
    pub ask: bool,

    /// Bypass Codex approvals and sandbox.
    #[arg(long, env = "CODEX_PAL_NO_SANDBOX", default_value_t = false)]
    pub no_sandbox: bool,

    /// Do not start codex-relay; still inject Codex config for the chosen port.
    #[arg(long, default_value_t = false)]
    pub no_start_relay: bool,

    /// Print the generated Codex argv and exit.
    #[arg(long, default_value_t = false)]
    pub print_codex_command: bool,

    /// Conservative context-window metadata injected for the selected model.
    #[arg(long, env = "CODEX_PAL_CONTEXT_WINDOW", default_value_t = 128_000)]
    pub context_window: u32,

    /// Extra arguments appended to the codex invocation.
    #[arg(last = true)]
    pub codex_args: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct ProviderArgs {
    /// Provider profile: deepseek, kimi, moonshot, qwen, mistral, groq, xai, openrouter, or openai.
    #[arg(long, env = "CODEX_PAL_PROVIDER", default_value = "openrouter")]
    pub provider: String,

    /// Override the provider's OpenAI-compatible /v1 upstream URL.
    #[arg(long, env = "CODEX_PAL_UPSTREAM")]
    pub upstream: Option<String>,

    /// Environment variable that contains the upstream provider API key.
    #[arg(long, env = "CODEX_PAL_API_KEY_ENV")]
    pub api_key_env: Option<String>,

    /// API key value for the relay. Prefer --api-key-env for normal use.
    #[arg(long, env = "CODEX_PAL_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Command::Status(args)) => {
            let status = relay_status(args.port);
            println!("{status}");
            Ok(())
        }
        Some(Command::Stop(args)) => {
            let result = stop_relay(args.port)?;
            println!("{result}");
            Ok(())
        }
        Some(Command::Config(args)) => {
            let profile = ProviderProfile::resolve(&args.provider)?;
            crate::relay::print_relay_config(&args.relay_bin, &profile)?;
            Ok(())
        }
        None => launch(cli.launch),
    }
}

fn launch(args: LaunchArgs) -> Result<()> {
    let profile = ProviderProfile::resolve(&args.provider)?;
    let relay = RelayRequest {
        relay_bin: args.relay_bin.clone(),
        provider: profile.clone(),
        port: args.port,
    };

    let relay_state = if profile.needs_relay() && !args.no_start_relay {
        Some(ensure_relay(&relay)?)
    } else {
        None
    };

    if let Some(state) = &relay_state {
        eprintln!("relay     {state}");
    } else if profile.needs_relay() {
        eprintln!(
            "relay     not started (--no-start-relay); expecting port {}",
            args.port
        );
    } else {
        eprintln!("relay     not needed for provider=openai");
    }

    let launch = CodexLaunch {
        codex_bin: args.codex_bin.clone(),
        provider: profile,
        port: args.port,
        model: args.model.clone(),
        ask: args.ask,
        no_sandbox: args.no_sandbox,
        context_window: args.context_window,
        extra_args: args.codex_args.clone(),
    };
    let command = build_codex_command(&launch)?;
    if args.print_codex_command {
        println!("{}", shell_join(&command.argv));
        return Ok(());
    }
    eprintln!("launching: {}", shell_join(&command.argv));
    let status = exec_codex(command)?;
    if status.success() {
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            if arg
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch))
            {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
