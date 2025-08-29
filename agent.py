#!/usr/bin/env uv run
# /// script
# dependencies = ["mirascope==1.25.6", "rich==14.1.0", "openai==1.102.0", "llm==0.27.1"]
# ///

import itertools
import os
import shlex
import subprocess
from pathlib import Path
from typing import Callable, List

assert os.getenv(
    "OPENROUTER_API_KEY"
), "Please set the OPENROUTER_API_KEY environment variable"

import readline  # pyright: ignore[reportUnusedImport]

from mirascope.core import openai
from mirascope.core.base.types import JsonableType
from openai import OpenAI
from openai.types.chat import (
    ChatCompletionMessageParam,
    ChatCompletionUserMessageParam,
    ChatCompletionSystemMessageParam,
)
from rich.console import Console
from rich.markdown import Markdown
from rich.live import Live
from pydantic import BaseModel

client = OpenAI(
    api_key=os.getenv("OPENROUTER_API_KEY"), base_url="https://openrouter.ai/api/v1"
)


def run_command(command: str, timeout: int, working_directory: str) -> str:
    """Execute a shell command safely and return its output."""
    try:
        # Security: Parse command safely, avoid shell=True
        args = shlex.split(command)

        # Whitelist allowed commands for security
        allowed_commands = {
            "awk",
            "cargo",
            "cat",
            "curl",
            "find",
            "git",
            "grep",
            "head",
            "ls",
            "make",
            "npm",
            "pip",
            "pwd",
            "python",
            "python3",
            "sed",
            "sort",
            "tail",
            "uniq",
            "uv",
            "wc",
            "wget",
        }

        if args[0] not in allowed_commands:
            return f"Command '{args[0]}' not allowed for security reasons. Allowed: {', '.join(sorted(allowed_commands))}"

        result = subprocess.run(
            args,
            capture_output=True,
            text=True,
            cwd=working_directory,
            timeout=timeout,
            shell=False,  # Critical for security
        )

        output = result.stdout
        if result.stderr:
            output += f"\nSTDERR: {result.stderr}"
        if result.returncode != 0:
            output += f"\nReturn code: {result.returncode}"

        return output

    except subprocess.TimeoutExpired:
        return f"Command timed out after {timeout} seconds"
    except Exception as e:
        return f"Error executing command: {e}"


def run_git_command(git_args: str, working_directory: str) -> str:
    """Execute git commands safely."""
    return run_command(
        f"git {git_args}", timeout=30, working_directory=working_directory
    )


# Git Integration Tools
def git_status(working_directory: str) -> str:
    """Get the current status of the git repository."""
    return run_git_command("status --porcelain", working_directory=working_directory)


def git_diff(file_path: str, working_directory: str) -> str:
    """Show git diff for the repository or specific file."""
    if file_path:
        return run_git_command(f"diff {file_path}", working_directory=working_directory)
    return run_git_command("diff", working_directory=working_directory)


# def git_commit(message: str, working_directory: str) -> str:
#     """Create a git commit with the given message."""
#     # First add all changes
#     add_result = run_git_command("add -A", working_directory=working_directory)
#     if "error" in add_result.lower() or "fatal" in add_result.lower():
#         return add_result
#     # Then commit with proper message escaping
#     escaped_message = shlex.quote(message)
#     return run_git_command(
#         f"commit -m {escaped_message}", working_directory=working_directory
#     )


def git_log(num_commits: int, working_directory: str) -> str:
    """Show recent git commit log."""
    return run_git_command(
        f"log --oneline -n {num_commits}", working_directory=working_directory
    )


# File Operations Tools
def read_file(path: str, working_directory: str) -> str:
    """Read the contents of a file at the given path."""
    wd_path = Path(working_directory)
    path_path = Path(path)

    if path_path.is_absolute():
        resolved_path = path_path
    else:
        resolved_path = wd_path / path_path

    try:
        # Security: Resolve path to prevent directory traversal
        resolved_path = resolved_path.resolve()

        with open(resolved_path, "r", encoding="utf-8") as f:
            content = f.read()
        return f"Successfully read {path}:\n\n{content}"
    except Exception as e:
        return f"Error reading file {path}: {e}"


def write_file(path: str, content: str, working_directory: str) -> str:
    """Write content to a file at the given path."""
    try:
        resolved_path = Path(working_directory) / path
        resolved_path = resolved_path.resolve()

        # Create parent directories if needed
        resolved_path.parent.mkdir(parents=True, exist_ok=True)

        with open(resolved_path, "w", encoding="utf-8") as f:
            f.write(content)
        return f"Successfully wrote to {path}"
    except Exception as e:
        return f"Error writing file {path}: {e}"


def search_files(pattern: str, directory: str, working_directory: str) -> str:
    """Search for files matching a pattern in the given directory."""
    try:
        search_path = Path(working_directory) / directory
        matches = list(search_path.rglob(pattern))

        if matches:
            relative_matches = [
                str(p.relative_to(search_path).resolve()) for p in matches
            ]
            return "\n".join(relative_matches)
        else:
            return "No matches found"
    except Exception as e:
        return f"Error searching files: {e}"


def list_directory(path: str, working_directory: str) -> str:
    """List the contents of a directory."""
    try:
        dir_path = Path(working_directory) / path
        dir_path = dir_path.resolve()

        items: list[str] = []
        for item in sorted(dir_path.iterdir()):
            if item.is_dir():
                items.append(f"{item.name}/")
            else:
                items.append(item.name)

        return "\n".join(items)
    except Exception as e:
        return f"Error listing directory {path}: {e}"


class CludCode(BaseModel):  # No relation to any french named coding agent.

    history: List[ChatCompletionMessageParam] = []

    @openai.call("openai/gpt-5-mini", client=client, stream=True)
    def _call(self, query: str) -> openai.OpenAIDynamicConfig:
        messages: list[ChatCompletionMessageParam] = [
            ChatCompletionSystemMessageParam(
                {
                    "role": "system",
                    "content": "You are a helpful coding assistant. Use tools to help the user with their requests.",
                }
            ),
            *self.history,
        ]
        if query:
            messages.append(
                ChatCompletionUserMessageParam({"role": "user", "content": query})
            )

        # Give the agent access to all our tools
        tools: list[Callable[..., str]] = [
            read_file,
            write_file,
            search_files,
            list_directory,
            git_status,
            git_diff,
            git_log,
        ]

        return {"messages": messages, "tools": tools}

    def _step(self, query: str) -> None:
        current_query = query

        md = ""
        with Live(Markdown(md), refresh_per_second=60) as content:
            with content.console.status("Thinking...", spinner="star") as status:
                for _ in range(10):  # top-level loop to allow multiple tool uses
                    stream = self._call(current_query)
                    if current_query:
                        self.history.append(
                            ChatCompletionUserMessageParam(
                                {"role": "user", "content": current_query}
                            )
                        )

                    parts: list[
                        tuple[openai.OpenAICallResponseChunk, openai.OpenAITool | None]
                    ] = []
                    stream_it = iter(stream)
                    while True:
                        part = next(stream_it)
                        parts.append(part)
                        (
                            chunk,
                            tool,
                        ) = part
                        if tool is not None or chunk.content.strip():
                            break

                    # Consume the stream to collect chunks and tools
                    tools_and_outputs: list[tuple[openai.OpenAITool, JsonableType]] = []

                    for chunk, tool in itertools.chain(parts, stream_it):
                        if tool is None:
                            assert chunk is not None, "Stream chunk is None"
                            if not chunk.content:
                                continue

                            md += chunk.content
                            if md.strip():
                                content.update(Markdown(md))
                        else:
                            assert tool is not None
                            name = tool._name()  # pyright: ignore[reportPrivateUsage]
                            status.update(f"• [bold]{name}[/bold]\n")
                            try:
                                output = tool.call()
                                tools_and_outputs.append((tool, output))
                                (line, *_) = output.split("\n", 1)[
                                    :100
                                ]  # Max 100 chars of the first line
                                status.update(f"Tool {name} output: {line}")
                            except Exception as e:
                                error_output = f"Tool error: {e}"
                                tools_and_outputs.append((tool, error_output))
                                status.update(f"Tool error: {error_output}")

                    # After consuming the stream, we can access message_param
                    self.history.append(stream.message_param)

                    if tools_and_outputs:
                        self.history += stream.tool_message_params(tools_and_outputs)
                        current_query = (
                            ""  # Continue loop with empty query for tool results
                        )
                    else:
                        return  # No more tools, exit loop

    def run(self) -> None:
        console = Console()

        try:
            while True:
                query = console.input("> ")
                if query.lower() in ("exit", "quit", "q", "stop", ":q", ":wq"):
                    break

                self._step(query)
        except (KeyboardInterrupt, EOFError):
            ...
        except Exception as e:
            print(f"Error: {e!r}")

        print("\nExiting...")


if __name__ == "__main__":
    agent = CludCode()
    agent.run()
