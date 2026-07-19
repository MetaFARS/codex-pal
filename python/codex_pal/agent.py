"""Async process wrapper for profile-based Codex agents."""

from __future__ import annotations

import asyncio
import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


@dataclass(frozen=True, slots=True)
class AgentResult:
    """Completed JSONL event stream from one Codex execution."""

    profile: str
    events: tuple[dict[str, Any], ...]
    stderr: str
    returncode: int


class AgentError(RuntimeError):
    """Raised when an agent process fails or returns invalid JSONL."""

    def __init__(self, message: str, result: AgentResult | None = None) -> None:
        super().__init__(message)
        self.result = result


@dataclass(frozen=True, slots=True)
class Agent:
    """A codex-pal profile bound to an optional workspace and relay port."""

    profile: str
    cwd: str | os.PathLike[str] | None = None
    port: int | None = None
    env: Mapping[str, str] = field(default_factory=dict)
    executable: str = "codex-pal"

    def __post_init__(self) -> None:
        if not self.profile.strip():
            raise ValueError("profile must not be empty")
        if self.port is not None and not 0 < self.port <= 65535:
            raise ValueError("port must be between 1 and 65535")

    async def run(
        self,
        prompt: str,
        *,
        args: Sequence[str] = (),
    ) -> AgentResult:
        """Run ``codex exec --json`` and return its decoded JSONL events."""

        command = self.command(args)
        environment = os.environ.copy()
        environment.update(self.env)
        process = await asyncio.create_subprocess_exec(
            *command,
            cwd=Path(self.cwd) if self.cwd is not None else None,
            env=environment,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        communication = asyncio.create_task(process.communicate(prompt.encode()))
        try:
            stdout, stderr = await asyncio.shield(communication)
        except asyncio.CancelledError:
            await asyncio.shield(communication)
            raise

        stderr_text = stderr.decode(errors="replace")
        events = _decode_events(self.profile, stdout, stderr_text, process.returncode)
        result = AgentResult(
            profile=self.profile,
            events=events,
            stderr=stderr_text,
            returncode=process.returncode,
        )
        if process.returncode != 0:
            detail = stderr_text.strip() or "no error output"
            raise AgentError(
                f"codex-pal profile {self.profile!r} exited with "
                f"{process.returncode}: {detail}",
                result,
            )
        return result

    def command(self, args: Sequence[str] = ()) -> tuple[str, ...]:
        """Return the argv used by :meth:`run`, primarily for inspection."""

        command = [self.executable, self.profile]
        if self.port is not None:
            command.extend(("--port", str(self.port)))
        command.extend(("exec", "--json"))
        command.extend(str(arg) for arg in args)
        command.append("-")
        return tuple(command)


@dataclass(frozen=True, slots=True)
class AgentTask:
    """One prompt to run with an agent."""

    agent: Agent
    prompt: str
    args: Sequence[str] = ()


async def run_parallel(tasks: Iterable[AgentTask]) -> list[AgentResult]:
    """Run independent profile tasks concurrently, preserving input order."""

    pending = [task.agent.run(task.prompt, args=task.args) for task in tasks]
    results = await asyncio.gather(*pending, return_exceptions=True)
    for result in results:
        if isinstance(result, BaseException):
            raise result
    return list(results)


def _decode_events(
    profile: str,
    stdout: bytes,
    stderr: str,
    returncode: int,
) -> tuple[dict[str, Any], ...]:
    events: list[dict[str, Any]] = []
    for line_number, raw_line in enumerate(stdout.splitlines(), start=1):
        if not raw_line.strip():
            continue
        try:
            event = json.loads(raw_line)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            partial = AgentResult(profile, tuple(events), stderr, returncode)
            raise AgentError(
                f"codex-pal profile {profile!r} returned invalid JSONL "
                f"on line {line_number}",
                partial,
            ) from error
        if not isinstance(event, dict):
            partial = AgentResult(profile, tuple(events), stderr, returncode)
            raise AgentError(
                f"codex-pal profile {profile!r} returned a non-object JSON "
                f"event on line {line_number}",
                partial,
            )
        events.append(event)
    return tuple(events)
