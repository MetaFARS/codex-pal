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

```bash
export DEEPSEEK_API_KEY=...
codex-pal --provider deepseek --model deepseek-chat

export DASHSCOPE_API_KEY=...
codex-pal --provider qwen --model qwen-plus

export OPENROUTER_API_KEY=...
codex-pal --provider openrouter --model moonshotai/kimi-k2.7-code
```

Custom OpenAI-compatible upstream:

```bash
export EXAMPLE_API_KEY=...
codex-pal \
  --provider custom \
  --upstream https://llm.example.com/v1 \
  --api-key-env EXAMPLE_API_KEY \
  --model vendor/model
```

Useful flags:

```bash
codex-pal status --port 4444
codex-pal stop --port 4444
codex-pal config --provider openrouter
codex-pal --provider deepseek --model deepseek-chat --print-codex-command
codex-pal --provider deepseek --model deepseek-chat --ask
codex-pal --provider deepseek --model deepseek-chat --no-sandbox
```

Extra arguments after `--` are appended to the `codex` invocation:

```bash
codex-pal --provider deepseek --model deepseek-chat -- --oss
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
