#!/usr/bin/env python3
"""Check that every English term receives at most one first-use annotation."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TERM_PATTERN = re.compile(r"\\termfirst\{([^{}]+)\}\{([^{}]+)\}")


@dataclass(frozen=True)
class FirstUse:
    english: str
    source: Path
    line: int


def source_files() -> list[Path]:
    """Return translated book sources in their final reading order."""
    files = [
        ROOT / "tex" / "frontmatter" / "titlepage.tex",
        ROOT / "tex" / "frontmatter" / "preface.tex",
    ]
    files.extend(
        ROOT / "tex" / "chapters" / f"chapter-{number:02d}.tex"
        for number in range(1, 33)
    )
    files.append(ROOT / "tex" / "appendices" / "appendix-a.tex")
    files.extend(sorted((ROOT / "tex" / "solutions" / "author").glob("*.tex")))
    files.extend(
        path
        for path in sorted((ROOT / "tex" / "appendices").glob("*.tex"))
        if path.name != "appendix-a.tex"
    )
    files.append(ROOT / "tex" / "solutions" / "translator-solutions.tex")
    files.extend(
        sorted((ROOT / "tex" / "solutions" / "translator").glob("*.tex"))
    )
    files.extend(sorted((ROOT / "tex" / "backmatter").glob("*.tex")))
    return [path for path in files if path.exists()]


def normalize_english(value: str) -> str:
    """Normalize an English key for duplicate detection."""
    return " ".join(value.replace("-", " ").split()).casefold()


def main() -> int:
    seen: dict[str, FirstUse] = {}
    problems: list[str] = []

    for path in source_files():
        contents = path.read_text(encoding="utf-8")
        for match in TERM_PATTERN.finditer(contents):
            use = FirstUse(
                english=" ".join(match.group(2).split()),
                source=path.relative_to(ROOT),
                line=contents.count("\n", 0, match.start()) + 1,
            )
            key = normalize_english(use.english)
            previous = seen.get(key)
            if previous is None:
                seen[key] = use
                continue
            problems.append(
                f"duplicate \\termfirst for {use.english!r}: "
                f"{previous.source}:{previous.line} and "
                f"{use.source}:{use.line}"
            )

    if problems:
        for problem in problems:
            print(f"ERROR: {problem}")
        return 1

    print(f"Verified {len(seen)} unique first-use terminology annotations.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
