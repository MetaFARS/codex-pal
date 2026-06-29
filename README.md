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
`PATH`.

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
codex-pal deepseek config --model deepseek-reasoner --port 4555
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
  --model deepseek-chat \
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
codex-pal run --provider deepseek --model deepseek-chat --print-codex-command
codex-pal run --provider deepseek --model deepseek-chat --ask
codex-pal run --provider deepseek --model deepseek-chat --no-sandbox
```

Extra arguments after `--` are appended to the `codex` invocation:

```bash
codex-pal run --provider deepseek --model deepseek-chat -- --oss
codex-pal deepseek -- --oss
```

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
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds all wheels and the sdist, publishes to PyPI via
Trusted Publishing, publishes the Rust crate to crates.io, and creates a
GitHub Release with the built artifacts.
