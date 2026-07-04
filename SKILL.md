# codex-pal

Codex CLI launcher — 通过 codex-relay 让 Codex 使用任何 OpenAI 兼容 provider。

## 能力

- 自动启动/复用本地 `codex-relay` sidecar
- 用 `-c` 注入临时 Codex 配置，不动 `~/.codex/config.toml`
- 内置 8 个 provider 的 upstream URL + API key 映射
- 人类友好 profile 模式 + 脚本用显式 `run` 模式

## 快速开始

```bash
# 安装
cargo install codex-pal codex-relay

# 启动
export DEEPSEEK_API_KEY=...
codex-pal deepseek
```

## 子命令

### Profile 模式

```
codex-pal <profile>                    — 启动（deepseek / qwen / openrouter ...）
codex-pal <profile> config --model X   — 配置 profile
codex-pal <profile> show               — 查看 profile 配置
codex-pal <profile> status             — relay 状态
codex-pal <profile> stop               — 停止 relay
codex-pal <profile> restart            — 重启 relay
codex-pal profiles                     — 列出所有 profile
codex-pal providers                    — 列出内置 provider
```

### 显式模式

```
codex-pal run \
  --provider deepseek \
  --model deepseek-v4-pro \
  --port 4444 \
  --approval never \
  --sandbox workspace-write
```

### 通用 flags

```
--approval never          — 不询问批准
--sandbox workspace-write — 沙箱模式
--ask                     — 交互式确认
--no-sandbox              — 无沙箱
--print-codex-command     — 只打印命令不执行
--oss                     — (追加到 codex) OSS 模式
```

Extra args after `--` 直达 `codex`：

```bash
codex-pal deepseek -- --oss
```

## 内置 Provider

| Provider | 上游 URL | API Key 环境变量 | 默认模型 |
|----------|----------|-----------------|----------|
| deepseek | api.deepseek.com/v1 | DEEPSEEK_API_KEY | deepseek-v4-pro |
| kimi | api.moonshot.cn/v1 | MOONSHOT_API_KEY | kimi-k2.7-code |
| qwen | dashscope.aliyuncs.com | DASHSCOPE_API_KEY | qwen3.7-max |
| mistral | api.mistral.ai/v1 | MISTRAL_API_KEY | mistral-medium-3-5+2 |
| groq | api.groq.com/openai/v1 | GROQ_API_KEY | openai/gpt-oss-120b |
| xai | api.x.ai/v1 | XAI_API_KEY | grok-4.3 |
| openrouter | openrouter.ai/api/v1 | OPENROUTER_API_KEY | openrouter/auto |
| openai | api.openai.com/v1 | OPENAI_API_KEY | gpt-5.5 |

自定义 provider：

```bash
export EXAMPLE_API_KEY=...
codex-pal work-llm config \
  --provider custom \
  --upstream https://llm.example.com/v1 \
  --api-key-env EXAMPLE_API_KEY \
  --model vendor/model
codex-pal work-llm
```

## 在 multi-agent 系统中的角色

```
Orchestrator
  │
  ├─ mono-client ready → 拿到下一个 theorem
  ├─ mono-client capsule → 获取 context
  ├─ coda-client clone → 检出代码
  │
  └─ codex-pal run --provider deepseek --model deepseek-v4-pro
       │
       └─ 启动 codex-relay + codex → Agent 开始翻译
```
