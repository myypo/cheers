#!/usr/bin/env python3
"""Keep the README example block in sync with examples/readme/src/main.rs."""

from pathlib import Path
import sys


def main() -> int:
    root = Path.cwd()
    readme = root / "README.md"
    example = root / "examples/readme/src/main.rs"
    start_marker = "<!-- readme-app:start -->"
    end_marker = "<!-- readme-app:end -->"

    readme_text = readme.read_text(encoding="utf-8")

    if start_marker not in readme_text or end_marker not in readme_text:
        print(
            f"missing README markers {start_marker!r} / {end_marker!r}",
            file=sys.stderr,
        )
        return 1

    before, rest = readme_text.split(start_marker, maxsplit=1)
    _, after = rest.split(end_marker, maxsplit=1)
    example_text = example.read_text(encoding="utf-8").rstrip()
    generated = (
        f"{start_marker}\n"
        f"```rust no_run\n"
        f"{example_text}\n"
        f"```\n"
        f"{end_marker}"
    )
    updated = before + generated + after

    if readme_text != updated:
        readme.write_text(updated, encoding="utf-8")
        print("README example was out of sync — updated README.md")
    else:
        print("README example is in sync")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
