import importlib.util
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import Mock

SCRIPT = Path(__file__).with_name("check_commit.py")
SPEC = importlib.util.spec_from_file_location("check_commit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
check_commit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_commit)


class CommitGuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cwd = Path("/repo")
        self.runner = Mock(
            side_effect=[
                CompletedProcess([], 0, "feature/task\n", ""),
                CompletedProcess([], 0, "src/main.rs\n", ""),
                CompletedProcess([], 0, "", ""),
                CompletedProcess([], 0, "", ""),
                CompletedProcess([], 0, "", ""),
            ]
        )

    def payload(self, command: str) -> dict[str, object]:
        return {
            "toolName": "bash",
            "toolArgs": {"command": command},
            "cwd": str(self.cwd),
        }

    def test_allows_non_commit_command(self) -> None:
        result = check_commit.inspect(self.payload("git status --short"), self.runner)
        self.assertEqual(result, {})
        self.runner.assert_not_called()

    def test_denies_commit_on_main(self) -> None:
        self.runner.side_effect = [
            CompletedProcess([], 0, "main\n", ""),
        ]
        result = check_commit.inspect(self.payload("git commit -m 'fix: repair export'"), self.runner)
        self.assertEqual(result["permissionDecision"], "deny")
        self.assertIn("main", result["permissionDecisionReason"])

    def test_ignores_quoted_commit_text(self) -> None:
        result = check_commit.inspect(self.payload("echo 'git commit -m bad'"), self.runner)
        self.assertEqual(result, {})
        self.runner.assert_not_called()

    def test_detects_commit_after_shell_separator(self) -> None:
        commands = check_commit.shell_commands("echo ready && git commit -m 'fix: repair export'")
        self.assertEqual(
            check_commit.commit_invocation(commands[1], self.cwd),
            self.cwd,
        )

    def test_detects_commit_after_shell_control_keyword(self) -> None:
        commands = check_commit.shell_commands("if true; then git commit -m 'fix: repair export'; fi")
        invocation = next(
            check_commit.commit_invocation(tokens, self.cwd)
            for tokens in commands
            if check_commit.commit_invocation(tokens, self.cwd) is not None
        )
        self.assertEqual(invocation, self.cwd)

    def test_runs_format_and_lint_for_code_changes(self) -> None:
        result = check_commit.inspect(self.payload("git commit -m 'fix: repair export'"), self.runner)
        self.assertEqual(result, {})
        self.assertIn(
            ["cargo", "format-check"],
            [call.args[0] for call in self.runner.call_args_list],
        )
        self.assertIn(
            ["cargo", "lint"],
            [call.args[0] for call in self.runner.call_args_list],
        )

    def test_denies_commit_when_quality_check_fails(self) -> None:
        self.runner.side_effect = [
            CompletedProcess([], 0, "feature/task\n", ""),
            CompletedProcess([], 0, "src/main.rs\n", ""),
            CompletedProcess([], 0, "", ""),
            CompletedProcess([], 0, "", ""),
            CompletedProcess([], 1, "", "clippy reported an issue"),
        ]
        result = check_commit.inspect(self.payload("git commit -m 'fix: repair export'"), self.runner)
        self.assertEqual(result["permissionDecision"], "deny")
        self.assertIn("cargo lint failed", result["permissionDecisionReason"])


if __name__ == "__main__":
    unittest.main()
