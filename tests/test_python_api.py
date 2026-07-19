import asyncio
import json
import unittest
from unittest.mock import patch

from codex_pal import Agent, AgentError, AgentTask, run_parallel


class FakeProcess:
    def __init__(
        self, stdout: bytes, stderr: bytes = b"", returncode: int | None = 0
    ):
        self._stdout = stdout
        self._stderr = stderr
        self.returncode = returncode
        self.input = None

    async def communicate(self, data):
        self.input = data
        return self._stdout, self._stderr


class AgentTests(unittest.IsolatedAsyncioTestCase):
    def test_builds_profile_command_without_changing_cli_defaults(self):
        agent = Agent("coder", cwd="/tmp/worktree")

        self.assertEqual(
            agent.command(("--ephemeral",)),
            ("codex-pal", "coder", "exec", "--json", "--ephemeral", "-"),
        )

    def test_explicit_port_is_a_profile_launch_override(self):
        agent = Agent("reviewer", port=4555)

        self.assertEqual(
            agent.command(),
            (
                "codex-pal",
                "reviewer",
                "--port",
                "4555",
                "exec",
                "--json",
                "-",
            ),
        )

    async def test_run_decodes_jsonl_and_sends_prompt_on_stdin(self):
        process = FakeProcess(
            b"\n".join(
                (
                    json.dumps({"type": "thread.started"}).encode(),
                    json.dumps({"type": "turn.completed"}).encode(),
                )
            )
        )
        with patch("asyncio.create_subprocess_exec", return_value=process):
            result = await Agent("coder").run("implement it")

        self.assertEqual(process.input, b"implement it")
        self.assertEqual(len(result.events), 2)
        self.assertEqual(result.events[-1]["type"], "turn.completed")

    async def test_nonzero_exit_raises_with_result(self):
        process = FakeProcess(b"", b"profile not found", 1)
        with patch("asyncio.create_subprocess_exec", return_value=process):
            with self.assertRaises(AgentError) as caught:
                await Agent("missing").run("task")

        self.assertEqual(caught.exception.result.returncode, 1)
        self.assertIn("profile not found", str(caught.exception))

    async def test_runs_multiple_profiles_in_parallel(self):
        async def run(agent, prompt, *, args=()):
            await asyncio.sleep(0)
            return agent.profile

        tasks = [
            AgentTask(Agent("architect"), "plan"),
            AgentTask(Agent("reviewer"), "review"),
        ]
        with patch.object(Agent, "run", run):
            results = await run_parallel(tasks)

        self.assertEqual(results, ["architect", "reviewer"])

    async def test_parallel_failure_waits_for_other_agents_to_finish(self):
        finished = []

        async def run(agent, prompt, *, args=()):
            if agent.profile == "architect":
                raise AgentError("planning failed")
            await asyncio.sleep(0.001)
            finished.append(agent.profile)

        tasks = [
            AgentTask(Agent("architect"), "plan"),
            AgentTask(Agent("reviewer"), "review"),
        ]
        with patch.object(Agent, "run", run):
            with self.assertRaises(AgentError):
                await run_parallel(tasks)

        self.assertEqual(finished, ["reviewer"])


if __name__ == "__main__":
    unittest.main()
