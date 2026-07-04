# codex-pal

`codex-pal` launches Codex CLI through
[`codex-relay`](https://github.com/MetaFARS/codex-relay) so Codex can use
OpenAI-compatible providers that do not natively implement the Responses API.

It is intentionally a small launcher:

- starts or reuses a local `codex-relay` sidecar;
- injects per-invocation Codex config with `-c`, leaving `~/.codex/config.toml` untouched;
- maps common providers to upstream URLs and API-key environment variables;
- hands the terminal to `codex`.

## Install

From crates.io:

```bash
cargo install codex-pal
```

From PyPI:

```bash
pipx install codex-pal
```

`codex-pal` expects `codex` and `codex-relay` to be installed and available on
`PATH`. The PyPI package declares `codex-relay` as a runtime dependency. The
Cargo package also declares the `codex-relay` crate dependency, but Cargo does
not install dependency binaries onto `PATH` when installing a binary crate; if
you install with Cargo, install both tools:

```bash
cargo install codex-pal codex-relay
```

## Usage

`codex-pal` has two interfaces:

- a complete, explicit interface for scripts and debugging;
- a profile interface for humans.

### Human-friendly profiles

```bash
export DEEPSEEK_API_KEY=...
codex-pal deepseek

export DASHSCOPE_API_KEY=...
codex-pal qwen

export OPENROUTER_API_KEY=...
codex-pal openrouter
```

The first run creates a profile under `~/.config/codex-pal/config.toml` when
the profile name matches a built-in provider. Later runs reuse it.

Configure or modify a profile:

```bash
codex-pal deepseek config --model deepseek-v4-pro --port 4555
codex-pal deepseek show
codex-pal profiles
codex-pal providers
codex-pal deepseek status
codex-pal deepseek stop
codex-pal deepseek restart
```

Custom profile:

```bash
export EXAMPLE_API_KEY=...
codex-pal work-llm config \
  --provider custom \
  --upstream https://llm.example.com/v1 \
  --api-key-env EXAMPLE_API_KEY \
  --model vendor/model
codex-pal work-llm
```

### Complete explicit interface

Use `run` when every setting should be supplied by arguments:

```bash
codex-pal run \
  --provider deepseek \
  --model deepseek-v4-pro \
  --port 4444 \
  --approval never \
  --sandbox workspace-write
```

Custom one-shot launch:

```bash
codex-pal run \
  --provider custom \
  --upstream https://llm.example.com/v1 \
  --api-key-env EXAMPLE_API_KEY \
  --model vendor/model
```

Useful flags:

```bash
codex-pal relay status --port 4444
codex-pal relay stop --port 4444
codex-pal relay-config --provider openrouter
codex-pal run --provider deepseek --model deepseek-v4-pro --print-codex-command
codex-pal run --provider deepseek --model deepseek-v4-pro --ask
codex-pal run --provider deepseek --model deepseek-v4-pro --no-sandbox
```

Arguments left after `codex-pal` consumes its profile or launch options are
appended to the `codex` invocation, so Codex subcommands and flags can be used
directly:

```bash
codex-pal run --provider deepseek --model deepseek-v4-pro exec --skip-git-repo-check "summarize this repo"
codex-pal deepseek exec --skip-git-repo-check "summarize this repo"
codex-pal deepseek --oss
```

Use `--` when you need to force a later argument to be handled by Codex even if
it looks like a `codex-pal` option:

```bash
codex-pal deepseek -- --model gpt-5.5
```

For relay-backed providers, `codex-pal` also injects a temporary Codex
`model_catalog_json` file so the in-session `/model` picker lists models for the
selected provider instead of only Codex's bundled OpenAI catalog. If Codex's
catalog command is unavailable, launch still continues with the chosen `-m`
model and prints a warning.

## Provider Profiles

| Provider | Upstream | API key env |
| --- | --- | --- |
| `deepseek` | `https://api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| `kimi`, `moonshot` | `https://api.moonshot.cn/v1` | `MOONSHOT_API_KEY` |
| `qwen`, `dashscope` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `DASHSCOPE_API_KEY` |
| `mistral` | `https://api.mistral.ai/v1` | `MISTRAL_API_KEY` |
| `groq` | `https://api.groq.com/openai/v1` | `GROQ_API_KEY` |
| `xai`, `grok` | `https://api.x.ai/v1` | `XAI_API_KEY` |
| `openrouter` | `https://openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |

Default models:

| Provider | Default model |
| --- | --- |
| `openai` | `gpt-5.5` |
| `deepseek` | `deepseek-v4-pro` |
| `kimi`, `moonshot` | `kimi-k2.7-code` |
| `qwen`, `dashscope` | `qwen3.7-max` |
| `mistral` | `mistral-medium-3-5+2` |
| `groq` | `openai/gpt-oss-120b` |
| `xai`, `grok` | `grok-4.3` |
| `openrouter` | `openrouter/auto` |

## Development

```bash
cargo test
cargo fmt --check
maturin build
```

## Release

Releases are tag-driven from GitHub Actions.

One-time setup:

1. Create a GitHub environment named `release`.
2. Add an environment secret named `CARGO_REGISTRY_TOKEN` with a crates.io API token.
3. On PyPI, create a pending Trusted Publisher for:
   - project: `codex-pal`
   - owner: `MetaFARS`
   - repository: `codex-pal`
   - workflow: `release.yml`
   - environment: `release`

Publish:

```bash
git tag v0.1.1
git push origin v0.1.1
```

The release workflow builds all wheels and the sdist, publishes to PyPI via
Trusted Publishing, publishes the Rust crate to crates.io, and creates a
GitHub Release with the built artifacts.
