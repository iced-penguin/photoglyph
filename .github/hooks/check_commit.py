#!/usr/bin/env python3

import json
import os
import re
import shlex
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable

COMMAND_SEPARATORS = {";", "&&", "||", "|", "&", "(", ")"}
COMMAND_PREFIXES = {"then", "do", "else", "elif"}
GIT_OPTIONS_WITH_VALUES = {
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
}
COMMIT_PREFIXES = ("feat", "fix", "docs", "refactor", "test", "chore")
CODE_PATHS = (".rs", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/")
Runner = Callable[..., subprocess.CompletedProcess[str]]


def permission(decision: str, reason: str | None = None) -> dict[str, str]:
    result = {"permissionDecision": decision}
    if reason:
        result["permissionDecisionReason"] = reason
    return result


def get_command(tool_args: Any) -> str | None:
    if isinstance(tool_args, dict):
        command = tool_args.get("command")
        return command if isinstance(command, str) else None
    if isinstance(tool_args, str):
        try:
            decoded = json.loads(tool_args)
        except json.JSONDecodeError:
            return tool_args
        if isinstance(decoded, dict):
            return get_command(decoded)
        return tool_args
    return None


def shell_commands(command: str) -> list[list[str]]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|")
    lexer.whitespace_split = True
    tokens = list(lexer)
    commands: list[list[str]] = []
    current: list[str] = []
    for token in tokens:
        if token in COMMAND_SEPARATORS or token in {"{", "}"}:
            if current:
                commands.append(current)
                current = []
        else:
            current.append(token)
    if current:
        commands.append(current)
    return commands


def commit_invocation(tokens: list[str], working_directory: Path) -> tuple[list[str], Path] | None:
    index = 0
    while index < len(tokens):
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", tokens[index]) or tokens[index] in COMMAND_PREFIXES:
            index += 1
        else:
            break
    if index >= len(tokens) or Path(tokens[index]).name not in {"git", "git.exe"}:
        return None

    repository = working_directory
    index += 1
    while index < len(tokens):
        option = tokens[index]
        if option in GIT_OPTIONS_WITH_VALUES and index + 1 < len(tokens):
            value = tokens[index + 1]
            if option == "-C":
                repository = (repository / value).resolve()
            index += 2
        elif option == "--no-pager" or option == "--literal-pathspecs":
            index += 1
        else:
            break
    if index < len(tokens) and tokens[index] == "commit":
        return tokens[index + 1 :], repository
    return None


def message_from_args(args: list[str], repository: Path, runner: Runner) -> str | None:
    messages: list[str] = []
    index = 0
    while index < len(args):
        argument = args[index]
        if argument in {"-m", "--message"} and index + 1 < len(args):
            messages.append(args[index + 1])
            index += 2
        elif argument.startswith("--message="):
            messages.append(argument.partition("=")[2])
            index += 1
        elif argument in {"-F", "--file"} and index + 1 < len(args):
            filename = args[index + 1]
            if filename == "-":
                return None
            try:
                messages.append((repository / filename).read_text(encoding="utf-8"))
            except OSError:
                return None
            index += 2
        elif argument.startswith("--file="):
            filename = argument.partition("=")[2]
            try:
                messages.append((repository / filename).read_text(encoding="utf-8"))
            except OSError:
                return None
            index += 1
        elif argument.startswith("-m") and argument != "-m":
            messages.append(argument[2:])
            index += 1
        else:
            index += 1

    if messages:
        return "\n\n".join(messages)
    if "--amend" in args or any(argument.startswith("--reuse-message=") for argument in args):
        result = runner(
            ["git", "log", "-1", "--format=%s"],
            cwd=repository,
            capture_output=True,
            text=True,
            check=False,
        )
        return result.stdout.strip() if result.returncode == 0 else None
    return None


def has_code_changes(repository: Path, runner: Runner) -> bool:
    result = runner(
        ["git", "diff", "--cached", "--name-only"],
        cwd=repository,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return True
    unstaged = runner(
        ["git", "diff", "--name-only"],
        cwd=repository,
        capture_output=True,
        text=True,
        check=False,
    )
    if unstaged.returncode != 0:
        return True
    paths = (result.stdout + "\n" + unstaged.stdout).splitlines()
    return any(path.endswith(".rs") or path in CODE_PATHS[1:] or path.startswith(".cargo/") for path in paths)


def run_quality_checks(repository: Path, runner: Runner) -> str | None:
    deadline = time.monotonic() + 240
    for command in (["cargo", "format-check"], ["cargo", "lint"]):
        try:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return "Commit checks exceeded their 240-second time limit."
            result = runner(
                command,
                cwd=repository,
                capture_output=True,
                text=True,
                timeout=remaining,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            return f"Could not complete {' '.join(command)} before committing: {error}"
        if result.returncode != 0:
            details = (result.stdout + result.stderr).strip()
            return f"{' '.join(command)} failed; fix the reported issues before committing.\n{details[-3000:]}"
    return None


def inspect(payload: Any, runner: Runner = subprocess.run) -> dict[str, str]:
    if not isinstance(payload, dict) or payload.get("toolName") != "bash":
        return {}
    command = get_command(payload.get("toolArgs"))
    if command is None:
        return {}

    working_directory = Path(payload.get("cwd") or os.getcwd()).resolve()
    try:
        commands = shell_commands(command)
    except ValueError:
        return permission("deny", "Unable to safely inspect this shell command before a possible commit.")

    invocations = [
        invocation
        for tokens in commands
        if (invocation := commit_invocation(tokens, working_directory)) is not None
    ]
    for args, repository in invocations:
        branch = runner(
            ["git", "branch", "--show-current"],
            cwd=repository,
            capture_output=True,
            text=True,
            check=False,
        )
        if branch.returncode != 0 or not branch.stdout.strip():
            return permission("deny", "Unable to verify the current Git branch; refusing to commit.")
        if branch.stdout.strip() == "main":
            return permission("deny", "Direct commits to main are not allowed. Use a task branch and a pull request.")

        message = message_from_args(args, repository, runner)
        if message is None:
            return permission(
                "deny",
                "Provide a commit message with -m or --message using the required Conventional Commit prefix.",
            )
        first_line = message.splitlines()[0].strip() if message.splitlines() else ""
        if not re.match(rf"^(?:{'|'.join(COMMIT_PREFIXES)}):\s+\S", first_line):
            return permission(
                "deny",
                "Commit messages must start with feat:, fix:, docs:, refactor:, test:, or chore: followed by a summary.",
            )

        if has_code_changes(repository, runner):
            error = run_quality_checks(repository, runner)
            if error:
                return permission("deny", error)
    return {}


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError as error:
        result = permission("deny", f"Commit guard received invalid hook input: {error}")
    else:
        result = inspect(payload)
    print(json.dumps(result, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
