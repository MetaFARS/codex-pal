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

#[cfg(unix)]
#[test]
fn managed_relay_port_rejects_a_different_upstream() {
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.toml");
    let state_dir = temp.path().join("state");
    let relay = temp.path().join("fake-relay");
    let codex = temp.path().join("fake-codex");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::fs::write(
        &relay,
        r#"#!/usr/bin/env python3
import argparse
import socket

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int, required=True)
parser.add_argument("--upstream")
args = parser.parse_args()

server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", args.port))
server.listen()
while True:
    connection, _ = server.accept()
    connection.close()
"#,
    )
    .unwrap();
    std::fs::write(
        &codex,
        r#"#!/usr/bin/env python3
import json
import sys

if sys.argv[1:4] == ["debug", "models", "--bundled"]:
    print(json.dumps({"models": [{"slug": "template"}]}))
"#,
    )
    .unwrap();
    std::fs::set_permissions(&relay, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&codex, std::fs::Permissions::from_mode(0o755)).unwrap();

    let run = |upstream: &str| {
        let mut command = codex_pal(&config_path);
        command
            .env("CODEX_PAL_STATE_DIR", &state_dir)
            .args([
                "run",
                "--provider",
                "custom",
                "--upstream",
                upstream,
                "--api-key-env",
                "EXAMPLE_API_KEY",
                "--model",
                "example-model",
                "--port",
                &port.to_string(),
                "--relay-bin",
                relay.to_str().unwrap(),
                "--codex-bin",
                codex.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };

    let first = run("https://one.example/v1");
    let second = run("https://two.example/v1");
    let mut stop = codex_pal(&config_path);
    let _ = stop
        .env("CODEX_PAL_STATE_DIR", &state_dir)
        .args(["relay", "stop", "--port", &port.to_string()])
        .output();

    assert!(first.status.success());
    assert!(!second.status.success());
    let stderr = String::from_utf8(second.stderr).unwrap();
    assert!(stderr.contains("is configured for upstream"));
    assert!(stderr.contains("use a different --port"));
}
