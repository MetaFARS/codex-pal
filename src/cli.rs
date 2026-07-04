use std::io::{self, IsTerminal, Write};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::codex::{
    ApprovalPolicy, CodexLaunch, SandboxMode, build_codex_command, exec_codex,
    write_provider_model_catalog,
};
use crate::config::{ConfigFile, ProfileConfig, config_path};
use crate::provider::{BUILTIN_PROVIDERS, ProviderProfile, default_model, is_builtin_provider};
use crate::relay::{RelayRequest, ensure_relay, relay_status, stop_relay};

#[derive(Debug, Parser)]
#[command(
    name = "codex-pal",
    version,
    about = "Launch Codex through codex-relay for OpenAI-compatible providers",
    disable_help_subcommand = true,
    long_about = "codex-pal launches Codex CLI through codex-relay for OpenAI-compatible providers.\n\nInterfaces:\n  codex-pal run --provider deepseek --model deepseek-v4-pro\n      Fully explicit, script-friendly launch.\n\n  codex-pal deepseek\n  codex-pal deepseek --model deepseek-v4-pro\n      Human-friendly profile launch. Built-in provider names create profiles on first use.\n\n  codex-pal deepseek config --model deepseek-v4-pro --port 4555\n  codex-pal deepseek show\n  codex-pal deepseek status\n  codex-pal deepseek stop\n  codex-pal deepseek restart\n      Profile management commands.\n\nProviders:\n  codex-pal providers\n      Show built-in providers and default models.\n\nRelay:\n  codex-pal relay status --port 4444\n  codex-pal relay stop --port 4444\n  codex-pal relay-config --provider openrouter\n      Inspect, stop, or print relay configuration.\n\nCodex integration:\n  codex-pal injects per-run Codex config with -c and leaves ~/.codex/config.toml untouched.\n  Relay-backed providers also get a temporary model_catalog_json so Codex's /model picker lists provider-specific models.\n\nRequirements:\n  codex and codex-relay must be installed and available on PATH."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Fully explicit launch path.
    Run(Box<RunArgs>),
    /// List saved profiles.
    Profiles,
    /// List built-in provider shortcuts.
    Providers,
    /// Print codex-pal config path.
    ConfigPath,
    /// Manage relay processes directly.
    Relay {
        #[command(subcommand)]
        command: RelayCommand,
    },
    /// Print a Codex config.toml snippet by delegating to codex-relay --print-config.
    RelayConfig(ConfigArgs),
    /// Human-friendly profile path, e.g. `codex-pal deepseek`.
    #[command(external_subcommand)]
    Profile(Vec<String>),
}

#[derive(Debug, Subcommand)]
pub enum RelayCommand {
    /// Print relay status for a port.
    Status(StatusArgs),
    /// Stop the managed relay for a port.
    Stop(StatusArgs),
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
pub struct RunArgs {
    #[command(flatten)]
    pub provider: ProviderArgs,

    #[command(flatten)]
    pub launch: LaunchOptions,
}

#[derive(Debug, Args, Clone)]
pub struct LaunchOptions {
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

    /// Approval policy passed to Codex config.
    #[arg(long, env = "CODEX_PAL_APPROVAL", default_value = "never")]
    pub approval: ApprovalPolicy,

    /// Sandbox mode passed to Codex.
    #[arg(long, env = "CODEX_PAL_SANDBOX", default_value = "workspace-write")]
    pub sandbox: SandboxMode,

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
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
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

#[derive(Debug, Parser, Clone)]
struct ProfileRunArgs {
    #[command(flatten)]
    launch: ProfileLaunchOptions,
}

#[derive(Debug, Args, Clone, Default)]
struct ProfileLaunchOptions {
    #[arg(short = 'm', long)]
    model: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    approval: Option<ApprovalPolicy>,
    #[arg(long)]
    sandbox: Option<SandboxMode>,
    #[arg(long, default_value_t = false)]
    ask: bool,
    #[arg(long, default_value_t = false)]
    no_sandbox: bool,
    #[arg(long, default_value_t = false)]
    no_start_relay: bool,
    #[arg(long, default_value_t = false)]
    print_codex_command: bool,
    #[arg(long)]
    context_window: Option<u32>,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    codex_args: Vec<String>,
}

#[derive(Debug, Parser, Clone)]
struct ProfileConfigArgs {
    #[arg(long)]
    provider: Option<String>,
    #[arg(short = 'm', long)]
    model: Option<String>,
    #[arg(long)]
    upstream: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    codex_bin: Option<String>,
    #[arg(long)]
    relay_bin: Option<String>,
    #[arg(long)]
    approval: Option<ApprovalPolicy>,
    #[arg(long)]
    sandbox: Option<SandboxMode>,
    #[arg(long)]
    context_window: Option<u32>,
    #[arg(long, default_value_t = false)]
    reset: bool,
    #[arg(long, default_value_t = false)]
    show: bool,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Command::Run(args)) => launch_explicit(*args),
        Some(Command::Profiles) => list_profiles(),
        Some(Command::Providers) => list_providers(),
        Some(Command::ConfigPath) => {
            println!("{}", config_path()?.display());
            Ok(())
        }
        Some(Command::Relay {
            command: RelayCommand::Status(args),
        }) => {
            println!("{}", relay_status(args.port));
            Ok(())
        }
        Some(Command::Relay {
            command: RelayCommand::Stop(args),
        }) => {
            println!("{}", stop_relay(args.port)?);
            Ok(())
        }
        Some(Command::RelayConfig(args)) => {
            let profile = ProviderProfile::resolve(&args.provider)?;
            crate::relay::print_relay_config(&args.relay_bin, &profile)
        }
        Some(Command::Profile(tokens)) => handle_profile(tokens),
        None => run_default_profile(),
    }
}

fn run_default_profile() -> Result<()> {
    let config = ConfigFile::load()?;
    let Some(default_profile) = config.default_profile else {
        print_setup_guide();
        return Ok(());
    };
    run_profile(default_profile, Vec::new())
}

fn launch_explicit(args: RunArgs) -> Result<()> {
    let provider = ProviderProfile::resolve(&args.provider)?;
    launch(provider, args.launch)
}

fn handle_profile(tokens: Vec<String>) -> Result<()> {
    let Some((name, rest)) = tokens.split_first() else {
        bail!("missing profile name");
    };
    if name == "help" {
        bail!("use `codex-pal --help` for help");
    }
    match rest.first().map(String::as_str) {
        Some("config") => configure_profile(name, rest[1..].to_vec()),
        Some("show") => show_profile(name),
        Some("status") => profile_status(name),
        Some("stop") => profile_stop(name),
        Some("restart") => {
            profile_stop(name)?;
            run_profile(name.clone(), rest[1..].to_vec())
        }
        _ => run_profile(name.clone(), rest.to_vec()),
    }
}

fn run_profile(name: String, args: Vec<String>) -> Result<()> {
    let run_args = ProfileRunArgs::try_parse_from(
        std::iter::once(name.as_str()).chain(args.iter().map(String::as_str)),
    )?;
    let (profile, saved) = load_or_create_profile(&name, &run_args.launch)?;
    if let Some(message) = profile_setup_message(&name, &profile) {
        println!("{message}");
        return Ok(());
    }
    let provider_args = ProviderArgs {
        provider: profile.provider.clone(),
        upstream: profile.upstream.clone(),
        api_key_env: profile.api_key_env.clone(),
        api_key: None,
    };
    let provider = ProviderProfile::resolve(&provider_args)?;
    let launch_options = launch_options_from_profile(&profile, &run_args.launch)?;
    if saved {
        eprintln!("profile  saved {name}");
    }
    launch(provider, launch_options)
}

fn configure_profile(name: &str, args: Vec<String>) -> Result<()> {
    let parsed = ProfileConfigArgs::try_parse_from(
        std::iter::once("config").chain(args.iter().map(String::as_str)),
    )?;
    if parsed.show {
        return show_profile(name);
    }
    let mut config = ConfigFile::load()?;
    if parsed.reset {
        config.profiles.remove(name);
        if config.default_profile.as_deref() == Some(name) {
            config.default_profile = None;
        }
        let path = config.save()?;
        println!("removed profile {name} from {}", path.display());
        return Ok(());
    }

    let mut profile = config
        .profiles
        .get(name)
        .cloned()
        .unwrap_or_else(|| ProfileConfig {
            provider: if is_builtin_provider(name) {
                name.to_string()
            } else {
                "custom".to_string()
            },
            ..ProfileConfig::default()
        });
    apply_profile_config_args(&mut profile, parsed);
    if profile.model.is_none() {
        profile.model = default_model(&profile.provider).map(str::to_string);
    }
    config.profiles.insert(name.to_string(), profile);
    config
        .default_profile
        .get_or_insert_with(|| name.to_string());
    let path = config.save()?;
    println!("saved profile {name} to {}", path.display());
    show_profile(name)
}

fn show_profile(name: &str) -> Result<()> {
    let config = ConfigFile::load()?;
    let Some(profile) = config.profiles.get(name) else {
        bail!("profile {name:?} not found");
    };
    println!("{}", toml::to_string_pretty(profile)?);
    Ok(())
}

fn profile_status(name: &str) -> Result<()> {
    let profile = required_profile(name)?;
    println!("{}", relay_status(profile.port.unwrap_or(4444)));
    Ok(())
}

fn profile_stop(name: &str) -> Result<()> {
    let profile = required_profile(name)?;
    println!("{}", stop_relay(profile.port.unwrap_or(4444))?);
    Ok(())
}

fn list_profiles() -> Result<()> {
    let config = ConfigFile::load()?;
    for (name, profile) in config.profiles {
        let marker = if config.default_profile.as_deref() == Some(&name) {
            "*"
        } else {
            " "
        };
        println!(
            "{marker} {name}\tprovider={}\tmodel={}",
            profile.provider,
            profile.model.unwrap_or_else(|| "(unset)".to_string())
        );
    }
    Ok(())
}

fn list_providers() -> Result<()> {
    for provider in BUILTIN_PROVIDERS {
        if *provider == "custom" {
            println!("{provider}\tconfigure with --upstream and --api-key-env");
        } else {
            println!(
                "{provider}\tdefault_model={}",
                default_model(provider).unwrap_or("(none)")
            );
        }
    }
    Ok(())
}

fn required_profile(name: &str) -> Result<ProfileConfig> {
    ConfigFile::load()?
        .profiles
        .get(name)
        .cloned()
        .with_context(|| format!("profile {name:?} not found"))
}

fn load_or_create_profile(
    name: &str,
    run_args: &ProfileLaunchOptions,
) -> Result<(ProfileConfig, bool)> {
    let mut config = ConfigFile::load()?;
    if let Some(profile) = config.profiles.get(name).cloned() {
        return Ok((profile, false));
    }
    if !is_builtin_provider(name) {
        bail!("profile {name:?} not found; configure it with `codex-pal {name} config ...`");
    }

    let model = run_args
        .model
        .clone()
        .or_else(|| default_model(name).map(str::to_string))
        .or_else(|| prompt("Model").ok().flatten());
    if model.is_none() && io::stdin().is_terminal() {
        bail!("model is required for first-time profile setup");
    }
    let profile = ProfileConfig {
        provider: name.to_string(),
        model,
        port: Some(run_args.port.unwrap_or(4444)),
        approval: Some(
            run_args
                .approval
                .unwrap_or(ApprovalPolicy::Never)
                .as_config_value()
                .to_string(),
        ),
        sandbox: Some(normalized_sandbox(run_args).as_cli_value().to_string()),
        context_window: Some(run_args.context_window.unwrap_or(128_000)),
        ..ProfileConfig::default()
    };
    config.profiles.insert(name.to_string(), profile.clone());
    config
        .default_profile
        .get_or_insert_with(|| name.to_string());
    config.save()?;
    Ok((profile, true))
}

fn launch_options_from_profile(
    profile: &ProfileConfig,
    run_args: &ProfileLaunchOptions,
) -> Result<LaunchOptions> {
    Ok(LaunchOptions {
        model: run_args
            .model
            .clone()
            .or_else(|| profile.model.clone())
            .or_else(|| default_model(&profile.provider).map(str::to_string)),
        codex_bin: profile
            .codex_bin
            .clone()
            .unwrap_or_else(|| "codex".to_string()),
        relay_bin: profile
            .relay_bin
            .clone()
            .unwrap_or_else(|| "codex-relay".to_string()),
        port: run_args.port.or(profile.port).unwrap_or(4444),
        approval: run_args.approval.unwrap_or_else(|| {
            profile
                .approval
                .as_deref()
                .unwrap_or("never")
                .parse()
                .unwrap_or(ApprovalPolicy::Never)
        }),
        sandbox: run_args.sandbox.unwrap_or_else(|| {
            profile
                .sandbox
                .as_deref()
                .unwrap_or("workspace-write")
                .parse()
                .unwrap_or(SandboxMode::WorkspaceWrite)
        }),
        ask: run_args.ask,
        no_sandbox: run_args.no_sandbox,
        no_start_relay: run_args.no_start_relay,
        print_codex_command: run_args.print_codex_command,
        context_window: run_args
            .context_window
            .or(profile.context_window)
            .unwrap_or(128_000),
        codex_args: if run_args.codex_args.is_empty() {
            profile.codex_args.clone()
        } else {
            run_args.codex_args.clone()
        },
    })
}

fn profile_setup_message(name: &str, profile: &ProfileConfig) -> Option<String> {
    if profile.provider.trim().is_empty() {
        return Some(format!(
            "profile {name:?} is missing a provider.\nConfigure it with:\n  codex-pal {name} config --provider deepseek --model deepseek-v4-pro\n\nRun `codex-pal --help` for more examples."
        ));
    }
    if profile.model.is_none() && default_model(&profile.provider).is_none() {
        return Some(format!(
            "profile {name:?} is missing a model.\nConfigure it with:\n  codex-pal {name} config --model vendor/model\n\nRun `codex-pal --help` for more examples."
        ));
    }
    if profile.provider == "custom" && profile.upstream.is_none() {
        return Some(format!(
            "custom profile {name:?} is missing an upstream URL.\nConfigure it with:\n  codex-pal {name} config --provider custom --upstream https://llm.example.com/v1 --api-key-env EXAMPLE_API_KEY --model vendor/model\n\nRun `codex-pal --help` for more examples."
        ));
    }
    if profile.provider == "custom" && profile.api_key_env.is_none() {
        return Some(format!(
            "custom profile {name:?} is missing an API key environment variable.\nConfigure it with:\n  codex-pal {name} config --api-key-env EXAMPLE_API_KEY --model vendor/model\n\nRun `codex-pal --help` for more examples."
        ));
    }
    None
}

fn print_setup_guide() {
    println!(
        "No default profile is configured.\n\nStart with a built-in provider:\n  export DEEPSEEK_API_KEY=...\n  codex-pal deepseek\n\nOr configure a custom profile:\n  codex-pal work-llm config --provider custom --upstream https://llm.example.com/v1 --api-key-env EXAMPLE_API_KEY --model vendor/model\n  codex-pal work-llm\n\nUseful discovery commands:\n  codex-pal providers\n  codex-pal profiles\n  codex-pal --help"
    );
}

fn apply_profile_config_args(profile: &mut ProfileConfig, args: ProfileConfigArgs) {
    if let Some(provider) = args.provider {
        profile.provider = provider;
    }
    if let Some(model) = args.model {
        profile.model = Some(model);
    }
    if let Some(upstream) = args.upstream {
        profile.upstream = Some(upstream);
    }
    if let Some(api_key_env) = args.api_key_env {
        profile.api_key_env = Some(api_key_env);
    }
    if let Some(port) = args.port {
        profile.port = Some(port);
    }
    if let Some(codex_bin) = args.codex_bin {
        profile.codex_bin = Some(codex_bin);
    }
    if let Some(relay_bin) = args.relay_bin {
        profile.relay_bin = Some(relay_bin);
    }
    if let Some(approval) = args.approval {
        profile.approval = Some(approval.as_config_value().to_string());
    }
    if let Some(sandbox) = args.sandbox {
        profile.sandbox = Some(sandbox.as_cli_value().to_string());
    }
    if let Some(context_window) = args.context_window {
        profile.context_window = Some(context_window);
    }
}

fn launch(provider: ProviderProfile, mut args: LaunchOptions) -> Result<()> {
    normalize_launch_options(&mut args);
    let relay = RelayRequest {
        relay_bin: args.relay_bin.clone(),
        provider: provider.clone(),
        port: args.port,
    };

    let relay_state = if provider.needs_relay() && !args.no_start_relay {
        Some(ensure_relay(&relay)?)
    } else {
        None
    };

    if let Some(state) = &relay_state {
        eprintln!("relay     {state}");
    } else if provider.needs_relay() {
        eprintln!(
            "relay     not started (--no-start-relay); expecting port {}",
            args.port
        );
    } else {
        eprintln!("relay     not needed for provider=openai");
    }

    let launch = CodexLaunch {
        codex_bin: args.codex_bin.clone(),
        model_catalog_json: if provider.needs_relay() {
            match write_provider_model_catalog(
                &args.codex_bin,
                &provider,
                args.port,
                args.model.as_deref(),
            ) {
                Ok(path) => path.map(|path| path.display().to_string()),
                Err(err) => {
                    eprintln!("model catalog unavailable: {err}");
                    None
                }
            }
        } else {
            None
        },
        provider,
        port: args.port,
        model: args.model.clone(),
        approval: args.approval,
        sandbox: args.sandbox,
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

fn normalize_launch_options(args: &mut LaunchOptions) {
    if args.no_sandbox {
        if args.ask {
            eprintln!("note: --ask ignored because --no-sandbox bypasses approvals too");
        }
        args.sandbox = SandboxMode::DangerFullAccess;
        args.approval = ApprovalPolicy::Never;
    } else if args.ask {
        args.approval = ApprovalPolicy::OnRequest;
    }
}

fn normalized_sandbox(args: &ProfileLaunchOptions) -> SandboxMode {
    if args.no_sandbox {
        SandboxMode::DangerFullAccess
    } else {
        args.sandbox.unwrap_or(SandboxMode::WorkspaceWrite)
    }
}

fn prompt(label: &str) -> Result<Option<String>> {
    if !io::stdin().is_terminal() {
        return Ok(None);
    }
    print!("{label}: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    Ok((!value.is_empty()).then_some(value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_explicit_run() {
        let cli = Cli::try_parse_from([
            "codex-pal",
            "run",
            "--provider",
            "deepseek",
            "--model",
            "deepseek-v4-pro",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Run(args)) => {
                assert_eq!(args.provider.provider, "deepseek");
                assert_eq!(args.launch.model.as_deref(), Some("deepseek-v4-pro"));
            }
            _ => panic!("expected run command"),
        }
    }

    #[test]
    fn explicit_run_forwards_unconsumed_codex_args_without_separator() {
        let cli = Cli::try_parse_from([
            "codex-pal",
            "run",
            "--provider",
            "deepseek",
            "exec",
            "--skip-git-repo-check",
            "hello",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Run(args)) => {
                assert_eq!(args.provider.provider, "deepseek");
                assert_eq!(
                    args.launch.codex_args,
                    vec!["exec", "--skip-git-repo-check", "hello"]
                );
            }
            _ => panic!("expected run command"),
        }
    }

    #[test]
    fn explicit_run_forwards_unknown_flag_as_codex_arg() {
        let cli =
            Cli::try_parse_from(["codex-pal", "run", "--provider", "deepseek", "--oss"]).unwrap();
        match cli.command {
            Some(Command::Run(args)) => {
                assert_eq!(args.provider.provider, "deepseek");
                assert_eq!(args.launch.codex_args, vec!["--oss"]);
            }
            _ => panic!("expected run command"),
        }
    }

    #[test]
    fn parses_profile_shortcut() {
        let cli =
            Cli::try_parse_from(["codex-pal", "deepseek", "--model", "deepseek-v4-pro"]).unwrap();
        match cli.command {
            Some(Command::Profile(tokens)) => {
                assert_eq!(tokens, vec!["deepseek", "--model", "deepseek-v4-pro"]);
            }
            _ => panic!("expected profile external subcommand"),
        }
    }

    #[test]
    fn parses_profile_shortcut_with_unknown_codex_flag() {
        let cli = Cli::try_parse_from(["codex-pal", "deepseek", "--oss"]).unwrap();
        match cli.command {
            Some(Command::Profile(tokens)) => {
                assert_eq!(tokens, vec!["deepseek", "--oss"]);
            }
            _ => panic!("expected profile external subcommand"),
        }
    }

    #[test]
    fn profile_run_forwards_unconsumed_codex_args_without_separator() {
        let run_args = ProfileRunArgs::try_parse_from([
            "deepseek",
            "--model",
            "deepseek-v4-pro",
            "exec",
            "--skip-git-repo-check",
            "hello",
        ])
        .unwrap();

        assert_eq!(run_args.launch.model.as_deref(), Some("deepseek-v4-pro"));
        assert_eq!(
            run_args.launch.codex_args,
            vec!["exec", "--skip-git-repo-check", "hello"]
        );
    }

    #[test]
    fn profile_run_forwards_unknown_flag_as_codex_arg() {
        let run_args = ProfileRunArgs::try_parse_from(["deepseek", "--oss"]).unwrap();

        assert_eq!(run_args.launch.codex_args, vec!["--oss"]);
    }

    #[test]
    fn profile_run_separator_forces_codex_args() {
        let run_args =
            ProfileRunArgs::try_parse_from(["deepseek", "--", "--model", "gpt-5.5"]).unwrap();

        assert_eq!(run_args.launch.model, None);
        assert_eq!(run_args.launch.codex_args, vec!["--model", "gpt-5.5"]);
    }

    #[test]
    fn custom_profile_missing_upstream_gets_setup_message() {
        let profile = ProfileConfig {
            provider: "custom".to_string(),
            model: Some("vendor/model".to_string()),
            api_key_env: Some("EXAMPLE_API_KEY".to_string()),
            ..ProfileConfig::default()
        };
        let message = profile_setup_message("work-llm", &profile).unwrap();
        assert!(message.contains("missing an upstream URL"));
        assert!(message.contains("codex-pal work-llm config"));
    }

    #[test]
    fn builtin_profile_without_model_uses_provider_default() {
        let profile = ProfileConfig {
            provider: "deepseek".to_string(),
            ..ProfileConfig::default()
        };
        let launch_options =
            launch_options_from_profile(&profile, &ProfileLaunchOptions::default()).unwrap();
        assert_eq!(launch_options.model.as_deref(), Some("deepseek-v4-pro"));
    }
}
