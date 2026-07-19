# codex-pal

[![PyPI](https://img.shields.io/pypi/v/codex-pal)](https://pypi.org/project/codex-pal/)
[![crates.io](https://img.shields.io/crates/v/codex-pal)](https://crates.io/crates/codex-pal)
[![CI](https://github.com/MetaFARS/codex-pal/actions/workflows/CI.yml/badge.svg)](https://github.com/MetaFARS/codex-pal/actions/workflows/CI.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/MetaFARS/codex-pal/blob/main/LICENSE)

**Bring your models to Codex—from one-command profiles to
Python-orchestrated multi-agent workflows.**

`codex-pal` lets Codex CLI use OpenAI-compatible Chat Completions providers
through [`codex-relay`](https://github.com/MetaFARS/codex-relay). For everyday
use, it is a small profile-based launcher that leaves your global Codex
configuration untouched. For automation, the same profiles can be composed as
asynchronous Python agents.

## Why codex-pal?

- **Native Codex experience** — keep the normal terminal UI, tools, sandbox,
  project context, and `codex exec --json` protocol.
- **No global config changes** — provider settings are injected per invocation;
  `~/.codex/config.toml` remains untouched.
- **Reusable provider profiles** — keep model, relay, approval, sandbox, and
  context settings under memorable names.
- **Safe multi-provider relays** — managed relays are serialized and identified
  by configuration, so one profile cannot silently use another provider.
- **Thin Python orchestration** — run profiles sequentially or concurrently
  without introducing another agent runtime or Python dependency stack.

## Quick start

Codex CLI must already be installed and available on `PATH`.

### One provider, one command

```bash
pipx install codex-pal

export DEEPSEEK_API_KEY=...
codex-pal deepseek
```

The first launch creates a reusable `deepseek` profile and then hands the
terminal to Codex. The same pattern works for `kimi`, `qwen`, `zai`, `mistral`,
`groq`, `xai`, and `openrouter`.

### Multiple models, programmable agents

Install `codex-pal` into the Python environment running the orchestrator:

```bash
python -m pip install codex-pal

export MOONSHOT_API_KEY=...
export DASHSCOPE_API_KEY=...

codex-pal architect config \
  --provider kimi \
  --model kimi-k3 \
  --port 4444 \
  --sandbox read-only

codex-pal reviewer config \
  --provider qwen \
  --model qwen3.7-max \
  --port 4445 \
  --sandbox read-only
```

```python
import asyncio
from pathlib import Path

from codex_pal import Agent, AgentTask, run_parallel


async def main():
    repo = Path("/path/to/project")
    results = await run_parallel([
        AgentTask(
            Agent("architect", cwd=repo),
            "Map the architecture and propose an implementation plan. Do not edit files.",
        ),
        AgentTask(
            Agent("reviewer", cwd=repo),
            "Find high-confidence correctness issues. Do not edit files.",
        ),
    ])
    for result in results:
        print(result.profile, result.events)


asyncio.run(main())
```

See the [Python Multi-Agent API](https://github.com/MetaFARS/codex-pal/blob/main/PYTHON_API.md)
for staged workflows, worktree isolation, event handling, custom providers, and
the complete API reference.

## How it works

```text
Profile (provider, model, policy)
              |
              v
         codex-pal
          /      \
         v        v
   Codex CLI -> codex-relay -> model provider
   native UX    Responses-to-Chat bridge
```

For each launch, `codex-pal` starts or reuses the appropriate relay, injects
Codex configuration with `-c`, and then runs Codex directly. Relay-backed
providers also get a temporary model catalog so Codex's `/model` picker lists
provider-specific models.

Managed local relays record their upstream identity. Concurrent launches are
serialized per port, and a differently configured profile is rejected instead
of being routed to the wrong provider. Remote relays remain available through
`--relay-url`.

## Install

### PyPI

For CLI use:

```bash
pipx install codex-pal
```

For the Python API, install into your application environment:

```bash
python -m pip install codex-pal
```

The PyPI package installs `codex-relay` as a runtime dependency. `codex-pal`
finds that dependency beside its own executable—including inside pipx's private
environment—before searching `PATH`. To expose `codex-relay` as a standalone
command too, use `pipx install codex-pal --include-deps`.

### crates.io

Cargo does not install dependency binaries onto `PATH`, so install both tools:

```bash
cargo install codex-pal codex-relay
```

## Profiles and CLI usage

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

### Remote relays

Use an existing remote `codex-relay` service instead of starting a local sidecar:

```bash
codex-pal run \
  --provider deepseek \
  --model deepseek-v4-pro \
  --relay-url https://relay.example.com

codex-pal deepseek --relay-url https://relay.example.com
codex-pal deepseek config --relay-url https://relay.example.com
```

`--relay-url` accepts either the relay root URL or its `/v1` base URL. When it
is set, `codex-pal` skips local relay process management and points Codex at
the remote relay.

### Forwarding Codex commands

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

## Python multi-agent API

The PyPI package includes a standard-library-only asyncio wrapper. Each `Agent`
uses an existing profile, keeping CLI and Python configuration in one place:

```python
from codex_pal import Agent

result = await Agent("architect", cwd="/path/to/project").run(
    "Analyze this repository and propose a refactoring plan. Do not edit files."
)
```

`Agent.run()` invokes the equivalent of `codex-pal <profile> exec --json -`,
sends the prompt over stdin, and returns decoded JSONL events. `cwd` can point
at a separate Git worktree for each writing agent. `run_parallel()` composes
independent profiles without replacing Codex or introducing a second agent
runtime.

Read the full [Python Multi-Agent API guide](https://github.com/MetaFARS/codex-pal/blob/main/PYTHON_API.md).

## Provider Profiles

| Provider | Upstream | API key env |
| --- | --- | --- |
| `deepseek` | `https://api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| `z`, `zai` | `https://api.z.ai/api/paas/v4` | `ZAI_API_KEY` |
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
| `z`, `zai` | `glm-5.2` |
| `kimi`, `moonshot` | `kimi-k3` |
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
