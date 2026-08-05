#!/usr/bin/env python3
"""Reject partial links and western spacing in Chinese structural references."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TEX_ROOT = ROOT / "tex"
RAW_REFERENCE = re.compile(r"\\(?:ref|pageref)\{")
DOUBLE_CHAPTER_PREFIX = re.compile(r"第\s*\\chapref\{")
VISIBLE_REFERENCE_SPACE = re.compile(
    r"(?:第\s*~|~\s*(?:章|节)|~\s*\\ref\*|\\ref\*\{[^}]+\}\s*~)"
)


def main() -> int:
    problems: list[str] = []

    for path in sorted(TEX_ROOT.rglob("*.tex")):
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if RAW_REFERENCE.search(line):
                problems.append(
                    f"{path.relative_to(ROOT)}:{line_number}: use \\cref, "
                    "\\chapref, \\taplsecref, or an enclosing \\hyperref "
                    "with \\ref*"
                )
            if DOUBLE_CHAPTER_PREFIX.search(line):
                problems.append(
                    f"{path.relative_to(ROOT)}:{line_number}: \\chapref already "
                    "includes the Chinese chapter prefix and suffix"
                )
            if VISIBLE_REFERENCE_SPACE.search(line):
                problems.append(
                    f"{path.relative_to(ROOT)}:{line_number}: Chinese structural "
                    "references must not use visible non-breaking spaces; use the "
                    "global reference commands"
                )

    if problems:
        print("Reference-link check failed:", file=sys.stderr)
        for problem in problems:
            print(f"- {problem}", file=sys.stderr)
        return 1

    print("Verified complete links and tight Chinese structural references.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
