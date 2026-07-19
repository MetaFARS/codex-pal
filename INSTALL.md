# Installing codex-pal

`codex-pal` enables OpenAI Codex CLI to use DeepSeek, Kimi, Qwen, Mistral, Groq, xAI, OpenRouter, and other model providers compatible with the OpenAI Chat Completions API.

Most users only need to install `codex-pal`. A compatible version of `codex-relay` is installed automatically, so the two programs do not need to be managed separately.

## Requirements

Before installation, make sure you have:

* Python 3.11 or later;
* OpenAI Codex CLI installed;
* macOS, Linux, or Windows 11;
* An API key for the model provider you want to use.

Current release targets include:

* macOS: Intel x86_64 and Apple Silicon ARM64;
* Linux: glibc x86_64/ARM64 and musl x86_64/ARM64;
* Windows: x86_64 and ARM64.

Both `codex-pal` and `codex-relay` require Python 3.11 or later. The current corresponding versions are `codex-pal 0.1.4` and `codex-relay 0.5.4`.

---

## macOS

The following steps apply to both Intel and Apple Silicon Macs.

### 1. Install Codex CLI

Use the official OpenAI installation script:

```bash
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

Alternatively, use Homebrew:

```bash
brew install --cask codex
```

Verify the installation:

```bash
codex --version
```

OpenAI officially provides three ways to install Codex CLI: the installation script, Homebrew, and npm.

### 2. Install Python and pipx

Using Homebrew:

```bash
brew install python pipx
pipx ensurepath
```

Reopen your terminal, or run:

```bash
exec zsh
```

Check the versions:

```bash
python3 --version
pipx --version
```

Python must be version 3.11 or later.

### 3. Install codex-pal

```bash
pipx install codex-pal
```

Verify the installation:

```bash
codex-pal --version
codex-pal providers
```

`codex-relay` is installed automatically as a runtime dependency. `codex-pal` looks for it in its own pipx environment first, so users do not need to expose `codex-relay` separately on `PATH`.

### 4. Configure and launch

For DeepSeek:

```bash
export DEEPSEEK_API_KEY="your-api-key"
codex-pal deepseek
```

For Qwen:

```bash
export DASHSCOPE_API_KEY="your-api-key"
codex-pal qwen
```

For OpenRouter:

```bash
export OPENROUTER_API_KEY="your-api-key"
codex-pal openrouter
```

The first time you run a built-in provider, `codex-pal` creates a corresponding local profile that can be reused on subsequent runs.

---

## Linux

The following steps apply to mainstream x86_64 and ARM64 Linux systems. Ubuntu, Debian, Fedora, Arch Linux, and Alpine Linux can all install the appropriate wheel.

### 1. Install Python 3.11 and pipx

On Ubuntu or Debian, use:

```bash
sudo apt update
sudo apt install -y python3 python3-venv pipx
```

Then run:

```bash
pipx ensurepath
exec "$SHELL" -l
```

Check the versions:

```bash
python3 --version
pipx --version
```

If your distribution provides a Python version older than 3.11, install a newer version using the method recommended by your distribution.

### 2. Install Codex CLI

Use the official OpenAI installation script:

```bash
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

Verify the installation:

```bash
codex --version
```

OpenAI currently lists macOS 12+ and Ubuntu 20.04+/Debian 10+ as the primary environments for Codex CLI.

### 3. Install codex-pal

```bash
pipx install codex-pal
```

Verify the installation:

```bash
codex-pal --version
codex-pal providers
```

The release process provides x86_64 and ARM64 wheels for both Linux glibc and musl environments, and tests installation in the corresponding manylinux or Alpine containers.

### 4. Configure a model provider

DeepSeek:

```bash
export DEEPSEEK_API_KEY="your-api-key"
codex-pal deepseek
```

Kimi:

```bash
export MOONSHOT_API_KEY="your-api-key"
codex-pal kimi
```

Qwen:

```bash
export DASHSCOPE_API_KEY="your-api-key"
codex-pal qwen
```

OpenRouter:

```bash
export OPENROUTER_API_KEY="your-api-key"
codex-pal openrouter
```

---

## Windows

Windows 11 and PowerShell are recommended. Current releases support both Windows x64 and Windows ARM64, with wheels tested on the corresponding native GitHub Actions runners.

### 1. Install Python

Install Python 3.11 or later. During installation, enable:

```text
Add Python to PATH
```

Check the version in PowerShell:

```powershell
py --version
```

If multiple Python versions are installed, list them with:

```powershell
py -0p
```

### 2. Install pipx

Run the following in PowerShell:

```powershell
py -3.11 -m pip install --user --upgrade pipx
py -3.11 -m pipx ensurepath
```

You can replace `3.11` with a later installed version, such as `3.12`, `3.13`, or `3.14`.

When the commands finish, close and reopen PowerShell, then verify the installation:

```powershell
pipx --version
```

### 3. Install Codex CLI

Use the official OpenAI PowerShell installation script:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
```

Verify the installation:

```powershell
codex --version
```

OpenAI currently provides a native Windows PowerShell installation script. Codex CLI can also be installed through npm.

### 4. Install codex-pal

```powershell
pipx install codex-pal
```

Verify the installation:

```powershell
codex-pal --version
codex-pal providers
```

If PowerShell cannot find `codex-pal` immediately, close and reopen the current window. You can also run:

```powershell
py -m pipx ensurepath
```

### 5. Configure and launch

DeepSeek:

```powershell
$env:DEEPSEEK_API_KEY = "your-api-key"
codex-pal deepseek
```

Qwen:

```powershell
$env:DASHSCOPE_API_KEY = "your-api-key"
codex-pal qwen
```

OpenRouter:

```powershell
$env:OPENROUTER_API_KEY = "your-api-key"
codex-pal openrouter
```

These environment variables apply only to the current PowerShell session. The API keys are not preserved after you close the window.

### Windows WSL2

When running under WSL2, treat WSL as a separate Linux system:

1. Install the Linux versions of Python, pipx, Codex CLI, and `codex-pal` inside WSL;
2. Set your API key inside WSL;
3. Do not mix Windows and WSL Python installations, pipx environments, or executables.

In other words, follow the Linux installation steps above from a WSL terminal.

---

## Verify the complete installation

Run the following on any platform:

```bash
codex --version
codex-pal --version
codex-pal providers
```

To view the relay version actually installed in the private `codex-pal` environment, run:

```bash
pipx runpip codex-pal show codex-relay
```

A compatible `codex-relay` version should be displayed.

You can also generate the Codex command without actually launching Codex to check your configuration:

```bash
codex-pal run \
  --provider deepseek \
  --model deepseek-v4-pro \
  --print-codex-command
```

The equivalent Windows PowerShell command is:

```powershell
codex-pal run `
  --provider deepseek `
  --model deepseek-v4-pro `
  --print-codex-command
```

---

## Upgrade

Upgrade `codex-pal`:

```bash
pipx upgrade codex-pal
```

Check the versions after upgrading:

```bash
codex-pal --version
pipx runpip codex-pal show codex-relay
```

To recreate a clean installation environment:

```bash
pipx reinstall codex-pal
```

---

## Uninstall

```bash
pipx uninstall codex-pal
```

This also removes the private pipx environment for `codex-pal` and its installed `codex-relay` dependency.

---

## Using codex-relay separately

Most users do not need to install `codex-relay` separately.

To run the relay as a standalone service, install it separately:

```bash
pipx install codex-relay
```

Then start it:

```bash
codex-relay \
  --upstream https://api.deepseek.com/v1 \
  --api-key "$DEEPSEEK_API_KEY" \
  --port 4446
```

On Windows PowerShell:

```powershell
codex-relay `
  --upstream https://api.deepseek.com/v1 `
  --api-key $env:DEEPSEEK_API_KEY `
  --port 4446
```

You can also have `pipx` expose the `codex-relay` command provided by the dependency on your system `PATH` when installing `codex-pal`:

```bash
pipx install codex-pal --include-deps
```

This is not required for typical `codex-pal` usage.

---

## Troubleshooting

### `codex-pal: command not found`

Run:

```bash
pipx ensurepath
```

Then reopen your terminal.

On Windows, use:

```powershell
py -m pipx ensurepath
```

### Python version is too old

Check the version:

```bash
python3 --version
```

On Windows:

```powershell
py --version
```

Python 3.11 or later is required.

### Codex cannot be found

Check the installation:

```bash
codex --version
```

If the command does not exist, install Codex CLI first.

### codex-relay cannot be found

With the standard installation command:

```bash
pipx install codex-pal
```

`codex-relay` is installed in the same private pipx environment, and `codex-pal` locates it automatically. You only need a separate installation or the `--include-deps` option when you want to run `codex-relay` directly from the command line.

### Check relay status

```bash
codex-pal relay status --port 4444
```

Stop the relay:

```bash
codex-pal relay stop --port 4444
```

### View existing profiles

```bash
codex-pal profiles
```

View a specific profile:

```bash
codex-pal deepseek show
```

Restart its relay:

```bash
codex-pal deepseek restart
```
