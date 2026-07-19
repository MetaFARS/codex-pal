# 安装 codex-pal

`codex-pal` 让 OpenAI Codex CLI 可以使用 DeepSeek、Kimi、Qwen、Mistral、Groq、xAI、OpenRouter，以及其他兼容 OpenAI Chat Completions API 的模型服务。

普通用户只需安装 `codex-pal`。安装过程中会自动安装匹配版本的 `codex-relay`，不需要分别管理两个程序。

## 系统要求

安装前请确认：

* Python 3.11 或更高版本；
* 已安装 OpenAI Codex CLI；
* macOS、Linux 或 Windows 11；
* 已取得所使用模型服务商的 API Key。

当前发行目标包括：

* macOS：Intel x86_64、Apple Silicon ARM64；
* Linux：glibc x86_64/ARM64、musl x86_64/ARM64；
* Windows：x86_64、ARM64。

`codex-pal` 与 `codex-relay` 均要求 Python 3.11 以上；当前对应版本为 `codex-pal 0.1.5` 和 `codex-relay 0.5.4`。

---

## macOS

以下步骤同时适用于 Intel Mac 和 Apple Silicon Mac。

### 1. 安装 Codex CLI

使用 OpenAI 官方安装脚本：

```bash
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

也可以使用 Homebrew：

```bash
brew install --cask codex
```

如果已经安装 Node.js 和 npm，也可以直接从 npm 安装 Codex CLI：

```bash
npm install -g @openai/codex
```

安装后检查：

```bash
codex --version
```

### 2. 安装 Python 和 pipx

使用 Homebrew：

```bash
brew install python pipx
pipx ensurepath
```

重新打开终端，或者执行：

```bash
exec zsh
```

检查版本：

```bash
python3 --version
pipx --version
```

Python 版本应为 3.11 或更高。

### 3. 安装 codex-pal

```bash
pipx install codex-pal
```

检查安装结果：

```bash
codex-pal --version
codex-pal providers
```

`codex-relay` 会作为运行时依赖自动安装。`codex-pal` 会优先在自身所在的 pipx 环境中查找它，不要求用户把 `codex-relay` 单独暴露到 `PATH`。

### 4. 配置并启动

以 DeepSeek 为例：

```bash
export DEEPSEEK_API_KEY="your-api-key"
codex-pal deepseek
```

以 Qwen 为例：

```bash
export DASHSCOPE_API_KEY="your-api-key"
codex-pal qwen
```

以 OpenRouter 为例：

```bash
export OPENROUTER_API_KEY="your-api-key"
codex-pal openrouter
```

第一次运行内置 provider 时，`codex-pal` 会创建对应的本地 profile；以后可以直接重复使用。

---

## Linux

以下步骤适用于主流 x86_64 和 ARM64 Linux。Ubuntu、Debian、Fedora、Arch Linux，以及 Alpine Linux 都可通过对应 wheel 安装。

### 1. 安装 Python 3.11 和 pipx

Ubuntu 或 Debian 可以使用：

```bash
sudo apt update
sudo apt install -y python3 python3-venv pipx
```

然后执行：

```bash
pipx ensurepath
exec "$SHELL" -l
```

检查版本：

```bash
python3 --version
pipx --version
```

如果发行版仓库中的 Python 低于 3.11，请通过该发行版推荐的方式安装较新的 Python。

### 2. 安装 Codex CLI

使用 OpenAI 官方安装脚本：

```bash
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

如果已经安装 Node.js 和 npm，也可以直接从 npm 安装 Codex CLI：

```bash
npm install -g @openai/codex
```

安装后检查：

```bash
codex --version
```

OpenAI 当前将 macOS 12+、Ubuntu 20.04+/Debian 10+ 列为 Codex CLI 的主要运行环境。

### 3. 安装 codex-pal

```bash
pipx install codex-pal
```

检查安装：

```bash
codex-pal --version
codex-pal providers
```

发布流程为 Linux glibc 和 musl 环境分别提供 x86_64、ARM64 wheel，并在对应 manylinux 或 Alpine 容器内执行安装测试。

### 4. 配置模型服务

DeepSeek：

```bash
export DEEPSEEK_API_KEY="your-api-key"
codex-pal deepseek
```

Kimi：

```bash
export MOONSHOT_API_KEY="your-api-key"
codex-pal kimi
```

Qwen：

```bash
export DASHSCOPE_API_KEY="your-api-key"
codex-pal qwen
```

OpenRouter：

```bash
export OPENROUTER_API_KEY="your-api-key"
codex-pal openrouter
```

---

## Windows

推荐使用 Windows 11 和 PowerShell。当前发布同时覆盖 Windows x64 与 Windows ARM64，并在相应 GitHub Actions 原生 runner 上安装 wheel 进行测试。

### 1. 安装 Python

安装 Python 3.11 或更高版本。安装时应启用：

```text
Add Python to PATH
```

在 PowerShell 中检查：

```powershell
py --version
```

如果系统安装了多个 Python 版本，可以查看：

```powershell
py -0p
```

### 2. 安装 pipx

在 PowerShell 中执行：

```powershell
py -3.11 -m pip install --user --upgrade pipx
py -3.11 -m pipx ensurepath
```

这里的 `3.11` 可以替换为已经安装的更高版本，例如 `3.12`、`3.13` 或 `3.14`。

执行完成后关闭并重新打开 PowerShell，然后检查：

```powershell
pipx --version
```

`pipx 1.16.0` 在 Windows 上有一个已知回归：如果 `%USERPROFILE%\.local\bin` 中包含其他工具放置的可执行文件，`pipx install` 可能在安装后处理阶段抛出 `UnicodeDecodeError`。遇到这种情况时，请先使用已确认可用的版本：

```powershell
py -3.11 -m pip install --user --force-reinstall "pipx==1.15.2"
```

然后重新执行安装。该错误来自 pipx 对共享命令目录的扫描，不表示 `codex-pal` wheel 或 Python 3.13 不兼容。

### 3. 安装 Codex CLI

使用 OpenAI 官方 PowerShell 安装脚本：

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
```

如果已经安装 Node.js 和 npm，也可以直接从 npm 安装 Codex CLI：

```powershell
npm install -g @openai/codex
```

检查安装：

```powershell
codex --version
```

通过 npm 安装时，实际入口通常是 `codex.cmd`。`codex-pal` 在 Windows 上会自动把默认名称 `codex` 解析为 `codex.cmd` 或 `codex.exe`，通常不需要额外配置。可以使用以下命令查看实际入口：

```powershell
Get-Command codex -All
```

### 4. 安装 codex-pal

```powershell
pipx install codex-pal
```

检查安装：

```powershell
codex-pal --version
codex-pal providers
py -3.11 -m pipx runpip codex-pal show codex-relay
```

标准安装会把兼容版本的 `codex-relay` 安装到同一个 pipx 私有环境。Windows 上它通常位于 `...\pipx\venvs\codex-pal\Scripts\codex-relay.exe`；`codex-pal` 会自动定位该文件，不需要再次独立安装 `codex-relay`，也不要求它出现在系统 `PATH` 中。

如果 PowerShell 暂时找不到 `codex-pal`，关闭当前窗口并重新打开；也可以重新执行：

```powershell
py -m pipx ensurepath
```

### 5. 配置并启动

DeepSeek：

```powershell
$env:DEEPSEEK_API_KEY = "your-api-key"
codex-pal deepseek
```

Qwen：

```powershell
$env:DASHSCOPE_API_KEY = "your-api-key"
codex-pal qwen
```

OpenRouter：

```powershell
$env:OPENROUTER_API_KEY = "your-api-key"
codex-pal openrouter
```

以上环境变量只在当前 PowerShell 会话中有效。关闭窗口后不会保留 API Key。

### Windows WSL2

在 WSL2 中运行时，应把 WSL 当作独立的 Linux 系统处理：

1. 在 WSL 内安装 Linux 版本的 Python、pipx、Codex CLI 和 `codex-pal`；
2. 在 WSL 内设置 API Key；
3. 不要混用 Windows 侧和 WSL 侧的 Python、pipx 或可执行文件。

也就是说，应在 WSL 终端中按照前面的 Linux 安装步骤执行。

---

## 验证完整安装

在任一平台执行：

```bash
codex --version
codex-pal --version
codex-pal providers
```

查看 `codex-pal` 私有环境中实际安装的 relay 版本：

```bash
pipx runpip codex-pal show codex-relay
```

正常情况下应显示兼容的 `codex-relay` 版本。

还可以生成但不实际启动 Codex 的命令，以检查配置：

```bash
codex-pal run \
  --provider deepseek \
  --model deepseek-v4-pro \
  --print-codex-command
```

Windows PowerShell 对应写法：

```powershell
codex-pal run `
  --provider deepseek `
  --model deepseek-v4-pro `
  --print-codex-command
```

---

## 更新

更新 `codex-pal`：

```bash
pipx upgrade codex-pal
```

更新后检查：

```bash
codex-pal --version
pipx runpip codex-pal show codex-relay
```

如果需要重新创建一个干净的安装环境：

```bash
pipx reinstall codex-pal
```

---

## 卸载

```bash
pipx uninstall codex-pal
```

这会同时删除 `codex-pal` 的 pipx 私有环境及其中安装的 `codex-relay` 依赖。

---

## 单独使用 codex-relay

普通用户不需要单独安装 `codex-relay`。

如果需要把 relay 作为独立服务运行，可以单独安装：

```bash
pipx install codex-relay
```

然后启动：

```bash
codex-relay \
  --upstream https://api.deepseek.com/v1 \
  --api-key "$DEEPSEEK_API_KEY" \
  --port 4446
```

Windows PowerShell：

```powershell
codex-relay `
  --upstream https://api.deepseek.com/v1 `
  --api-key $env:DEEPSEEK_API_KEY `
  --port 4446
```

也可以让 `pipx` 在安装 `codex-pal` 时同时把依赖提供的 `codex-relay` 命令暴露到系统 `PATH`：

```bash
pipx install codex-pal --include-deps
```

对于一般的 `codex-pal` 使用场景，这一步不是必需的。

---

## 常见问题

### `codex-pal: command not found`

执行：

```bash
pipx ensurepath
```

然后重新打开终端。

Windows 可以使用：

```powershell
py -m pipx ensurepath
```

### pipx 抛出 `UnicodeDecodeError`

如果 traceback 来自 `pipx\commands\common.py`，并在 `os.fsdecode` 或 `_copy_launcher_targets_venv` 附近以 `UnicodeDecodeError` 结束，通常是 `pipx 1.16.0` 扫描 `%USERPROFILE%\.local\bin` 中其他工具的可执行文件时触发的已知问题，不是 `codex-pal` 安装包错误。

先确认版本：

```powershell
py -3.11 -m pipx --version
```

如果是 `1.16.0`，暂时降级后重装：

```powershell
py -3.11 -m pip install --user --force-reinstall "pipx==1.15.2"
py -3.11 -m pipx uninstall codex-pal
py -3.11 -m pipx install codex-pal
```

如果卸载提示 `codex-pal` 尚未安装，可以忽略并继续执行安装命令。

### Python 版本过低

检查：

```bash
python3 --version
```

Windows：

```powershell
py --version
```

需要 Python 3.11 或更高版本。

### 找不到 Codex

检查：

```bash
codex --version
```

如果命令不存在，请先完成 Codex CLI 的安装。

Windows 可以进一步检查实际入口：

```powershell
Get-Command codex -All
```

如果入口是 `codex.cmd`，当前版本的 `codex-pal` 会自动解析。需要显式覆盖时可以使用：

```powershell
$env:CODEX_PAL_CODEX_BIN = "codex.cmd"
codex-pal deepseek
```

### 找不到 codex-relay

使用标准的：

```bash
pipx install codex-pal
```

时，`codex-relay` 已安装在同一个 pipx 私有环境中，`codex-pal` 会自行查找它。在 Windows 上检查依赖是否存在：

```powershell
py -3.11 -m pipx runpip codex-pal show codex-relay
```

如果可以显示版本信息，就不应再独立安装。只有需要直接从命令行运行 `codex-relay` 时，才需要单独安装，或者在初次安装时使用 `pipx install codex-pal --include-deps` 将依赖命令暴露到 `PATH`。

### 查看 relay 状态

```bash
codex-pal relay status --port 4444
```

停止 relay：

```bash
codex-pal relay stop --port 4444
```

### 查看已有 profile

```bash
codex-pal profiles
```

查看某个 profile：

```bash
codex-pal deepseek show
```

重新启动对应 relay：

```bash
codex-pal deepseek restart
```
