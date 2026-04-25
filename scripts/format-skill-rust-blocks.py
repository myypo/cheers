#!/usr/bin/env python3
"""Format Rust fenced code blocks in skill Markdown files with cargo-cheers."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess
import sys

RUST_FENCE = re.compile(
    r"(?ms)^(```[ \t]*(?:rust|rs)\b[^\n]*\n)(.*?)(^```[ \t]*(?:\n|$))"
)


def format_rust(source: str, path: Path) -> str:
    result = subprocess.run(
        ["cargo-cheers", "fmt", "--stdin", "--rustfmt"],
        input=source,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        sys.stderr.write(f"{path}: failed to format Rust markdown block\n")
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)

    formatted = result.stdout
    if source.endswith("\n") and not formatted.endswith("\n"):
        formatted += "\n"
    return formatted


def format_file(path: Path) -> bool:
    if not path.exists() or path.suffix != ".md":
        return False

    text = path.read_text(encoding="utf-8")

    def replace(match: re.Match[str]) -> str:
        opener, code, closer = match.groups()
        if not code.strip():
            return match.group(0)
        return opener + format_rust(code, path) + closer

    updated = RUST_FENCE.sub(replace, text)
    if updated == text:
        return False

    path.write_text(updated, encoding="utf-8")
    return True


def main(paths: list[str]) -> int:
    for raw_path in paths:
        path = Path(raw_path)
        if format_file(path):
            print(f"formatted Rust code blocks in {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
