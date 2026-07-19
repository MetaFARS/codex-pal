use std::path::Path;
use std::process::Command;

fn codex_pal(config_path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codex-pal"));
    command.env("CODEX_PAL_CONFIG", config_path);
    command
}

#[test]
fn no_args_without_default_profile_prints_setup_guide() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("missing.toml");

    let output = codex_pal(&config_path).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("No default profile is configured."));
    assert!(stdout.contains("codex-pal deepseek"));
    assert!(stdout.contains("codex-pal --help"));
}

#[test]
fn no_args_with_incomplete_default_profile_prints_profile_setup_guide() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
default_profile = "work-llm"

[profiles.work-llm]
provider = "custom"
model = "vendor/model"
api_key_env = "EXAMPLE_API_KEY"
"#,
    )
    .unwrap();

    let output = codex_pal(&config_path).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("custom profile \"work-llm\" is missing an upstream URL"));
    assert!(stdout.contains("codex-pal work-llm config"));
}

#[test]
fn dash_dash_help_is_the_help_entrypoint() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("missing.toml");

    let output = codex_pal(&config_path).arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("codex-pal launches Codex CLI"));
    assert!(stdout.contains("codex-pal deepseek config --model deepseek-v4-pro"));
    assert!(!stdout.contains("\n  help"));
}

#[test]
fn help_profile_name_points_to_dash_dash_help() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("missing.toml");

    let output = codex_pal(&config_path).arg("help").output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("use `codex-pal --help` for help"));
}

#[test]
fn providers_lists_current_default_models() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("missing.toml");

    let output = codex_pal(&config_path).arg("providers").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("deepseek\tdefault_model=deepseek-v4-pro"));
    assert!(stdout.contains("z\tdefault_model=glm-5.2"));
    assert!(stdout.contains("zai\tdefault_model=glm-5.2"));
    assert!(stdout.contains("kimi\tdefault_model=kimi-k3"));
    assert!(stdout.contains("openrouter\tdefault_model=openrouter/auto"));
}
