# Python Multi-Agent API

`codex-pal` includes a small, standard-library-only Python API for running
multiple Codex profiles as agents. Profiles remain the source of truth for
providers, models, relay ports, and Codex permissions; Python only starts and
coordinates those profiles.

## Installation

Install `codex-pal` into the Python environment that runs the orchestrator:

```bash
python -m pip install codex-pal
```

`pipx install codex-pal` is still recommended for CLI-only use, but a pipx
environment is isolated from project Python environments. Use `pip` in a venv
when importing `codex_pal` from application code.

Codex CLI must also be installed and available on `PATH`.

## Configure a multi-profile environment

The following example uses three providers and gives each local relay a unique
port. This prevents one profile from accidentally sharing another provider's
relay.

```bash
export MOONSHOT_API_KEY=...
export DEEPSEEK_API_KEY=...
export DASHSCOPE_API_KEY=...

# Read-only planning agent.
codex-pal architect config \
  --provider kimi \
  --model kimi-k3 \
  --port 4444 \
  --sandbox read-only

# Writing implementation agent.
codex-pal coder config \
  --provider deepseek \
  --model deepseek-v4-pro \
  --port 4445 \
  --sandbox workspace-write

# Read-only review agent.
codex-pal reviewer config \
  --provider qwen \
  --model qwen3.7-max \
  --port 4446 \
  --sandbox read-only
```

Inspect the resulting setup:

```bash
codex-pal profiles
codex-pal architect show
codex-pal coder show
codex-pal reviewer show
```

The Python API uses these profiles unchanged. `Agent(..., port=4555)` can
override a port for one invocation, but normally ports should be configured in
the profiles and omitted from Python code.

### Relay port safety

Managed relays record their upstream URL and API-key environment variable. A
profile attempting to use a port occupied by a differently configured managed
relay fails with an actionable error instead of sending work to the wrong
provider.

Use a different profile port, or explicitly stop the old relay:

```bash
codex-pal reviewer stop
# or, when recovering a stale startup reservation:
codex-pal relay stop --port 4446
```

## API reference

### `Agent`

```python
Agent(
    profile: str,
    cwd: str | os.PathLike[str] | None = None,
    port: int | None = None,
    env: Mapping[str, str] = {},  # an empty mapping is created per Agent
    executable: str = "codex-pal",
)
```

- `profile`: existing codex-pal profile name.
- `cwd`: working directory inherited by Codex. Use separate Git worktrees for
  agents that may write files concurrently.
- `port`: optional one-run profile port override.
- `env`: environment additions or overrides, merged with the current process.
- `executable`: alternate `codex-pal` executable, mainly useful in tests or
  custom installations.

Run one task:

```python
agent = Agent("architect", cwd="/work/project")
result = await agent.run(
    "Analyze the authentication flow.",
    args=("--ephemeral",),
)
```

`args` are appended to `codex exec --json`. For example:

```python
result = await agent.run(
    "Summarize this directory.",
    args=("--skip-git-repo-check", "--ephemeral"),
)
```

The prompt is passed over stdin rather than placed in the process argument
list.

### `AgentResult`

```python
AgentResult(
    profile: str,
    events: tuple[dict[str, Any], ...],
    stderr: str,
    returncode: int,
)
```

`events` contains the decoded JSONL objects emitted by `codex exec --json`.
These event objects belong to the installed Codex CLI protocol; codex-pal keeps
them raw rather than defining a second, potentially incompatible event schema.

For example, current Codex versions emit completed agent messages as item
events. Applications should tolerate additional event types:

```python
from codex_pal import AgentResult


def agent_messages(result: AgentResult) -> list[str]:
    messages = []
    for event in result.events:
        item = event.get("item", {})
        if (
            event.get("type") == "item.completed"
            and item.get("type") == "agent_message"
            and isinstance(item.get("text"), str)
        ):
            messages.append(item["text"])
    return messages
```

### `AgentTask` and `run_parallel`

```python
AgentTask(agent: Agent, prompt: str, args: Sequence[str] = ())

await run_parallel(tasks: Iterable[AgentTask]) -> list[AgentResult]
```

Results preserve input order. Tasks are independent: all started agents are
allowed to finish before an `AgentError` from one task is surfaced. This avoids
terminating another agent in the middle of a file change.

### Errors and cancellation

`Agent.run()` raises `AgentError` when codex-pal exits unsuccessfully or stdout
is not valid JSONL. `AgentError.result` contains the available return code,
stderr, and any events decoded before the error.

The initial API deliberately does not force-kill Codex processes. If a running
`Agent.run()` coroutine is cancelled, it waits for that Codex execution to
finish before propagating cancellation. Enforce stronger lifecycle policies in
an external sandbox or process supervisor when required.

## Example: parallel independent agents

This is useful when several models independently analyze the same repository.
Read-only agents can share a workspace:

```python
import asyncio
from pathlib import Path

from codex_pal import Agent, AgentTask, run_parallel


async def main() -> None:
    repository = Path("/work/project")
    tasks = [
        AgentTask(
            Agent("architect", cwd=repository),
            "Map the architecture and identify risky coupling. Do not edit files.",
        ),
        AgentTask(
            Agent("reviewer", cwd=repository),
            "Review the repository for high-confidence correctness bugs. Do not edit files.",
        ),
    ]

    for result in await run_parallel(tasks):
        print(f"\n== {result.profile} ==")
        for event in result.events:
            print(event)


asyncio.run(main())
```

## Example: planner → coder → reviewer

Use separate Git worktrees when an agent writes files:

```bash
git worktree add /tmp/project-coder -b agent/coder HEAD
```

The orchestration itself can remain ordinary Python:

```python
import asyncio
import json
from pathlib import Path

from codex_pal import Agent


def transcript(events: tuple[dict, ...]) -> str:
    return "\n".join(json.dumps(event, ensure_ascii=False) for event in events)


async def main() -> None:
    source = Path("/work/project")
    coder_worktree = Path("/tmp/project-coder")

    architect = Agent("architect", cwd=source)
    coder = Agent("coder", cwd=coder_worktree)
    reviewer = Agent("reviewer", cwd=coder_worktree)

    plan = await architect.run(
        "Analyze the requested feature and produce a concrete implementation plan. "
        "Do not modify files."
    )

    implementation = await coder.run(
        "Implement the following plan, run focused tests, and summarize the changes.\n\n"
        + transcript(plan.events)
    )

    review = await reviewer.run(
        "Review the current worktree changes. Report only high-confidence blockers.\n\n"
        + transcript(implementation.events)
    )

    print(transcript(review.events))


asyncio.run(main())
```

The reviewer above runs after the coder, so it can safely inspect the coder's
worktree. Do not run multiple writing agents concurrently in one worktree.

## Custom providers and remote relays

Custom profiles work the same way:

```bash
export EXAMPLE_API_KEY=...
codex-pal specialist config \
  --provider custom \
  --upstream https://llm.example.com/v1 \
  --api-key-env EXAMPLE_API_KEY \
  --model vendor/model \
  --port 4450
```

A profile configured with `--relay-url` uses that remote relay and does not
start or manage a local relay process. Port separation is only relevant to
locally managed relays.
