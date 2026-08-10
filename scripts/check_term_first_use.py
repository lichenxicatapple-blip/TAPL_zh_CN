#!/usr/bin/env python3
"""Check and index whole-book first-use English terminology annotations."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TERMINOLOGY = ROOT / "notes" / "terminology.md"
INDEX_START = "<!-- term-first-use:index:start -->"
INDEX_END = "<!-- term-first-use:index:end -->"
TERM_PATTERN = re.compile(r"\\termfirst\{([^{}]+)\}\{([^{}]+)\}")
HEADING_PATTERN = re.compile(
    r"\\(?:section|subsection|subsubsection)\*?\{([^{}]+)\}"
)
MANUAL_TABLE_HEADER = "| 英文 | 当前中文译法 | 状态 | 备注 |"


@dataclass(frozen=True)
class FirstUse:
    chinese: str
    english: str
    location: str
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


def base_location(path: Path) -> str:
    """Give a stable human-readable location for a book source file."""
    if path.name == "preface.tex":
        return "前言"
    chapter = re.fullmatch(r"chapter-(\d+)\.tex", path.name)
    if chapter:
        if path.parent.name == "author":
            return f"作者解答（第 {int(chapter.group(1))} 章）"
        if path.parent.name == "translator":
            return f"译者附录（第 {int(chapter.group(1))} 章）"
        return f"第 {int(chapter.group(1))} 章"
    if path.name == "translator-solutions.tex":
        return "译者附录"
    if path.parent.name == "appendices":
        return f"附录（{path.stem}）"
    if path.parent.name == "backmatter":
        return f"后置材料（{path.stem}）"
    return f"前置材料（{path.stem}）"


def plain_heading(value: str) -> str:
    """Remove lightweight TeX markup from a heading used in the index."""
    value = re.sub(r"\\[A-Za-z@]+", "", value)
    value = value.replace("{", "").replace("}", "")
    return " ".join(value.split())


def normalize_english(value: str) -> str:
    """Normalize an English key for duplicate detection."""
    return " ".join(value.replace("-", " ").split()).casefold()


def scan_first_uses() -> tuple[list[FirstUse], list[str]]:
    """Collect annotations and report duplicate English keys."""
    uses: list[FirstUse] = []
    seen: dict[str, FirstUse] = {}
    problems: list[str] = []

    for path in source_files():
        contents = path.read_text(encoding="utf-8")
        headings = list(HEADING_PATTERN.finditer(contents))
        heading_index = 0
        heading = ""

        # Read the whole file so an annotation may wrap across source lines.
        for match in TERM_PATTERN.finditer(contents):
            while (
                heading_index < len(headings)
                and headings[heading_index].start() < match.start()
            ):
                heading = plain_heading(headings[heading_index].group(1))
                heading_index += 1

            location = base_location(path)
            if heading:
                location = f"{location}·{heading}"
            use = FirstUse(
                chinese=" ".join(match.group(1).split()),
                english=" ".join(match.group(2).split()),
                location=location,
                source=path.relative_to(ROOT),
                line=contents.count("\n", 0, match.start()) + 1,
            )
            key = normalize_english(use.english)
            previous = seen.get(key)
            if previous is not None:
                problems.append(
                    "duplicate \\\\termfirst for "
                    f"{use.english!r}: "
                    f"{previous.source}:{previous.line} and "
                    f"{use.source}:{use.line}"
                )
            else:
                seen[key] = use
                uses.append(use)

    return uses, problems


def markdown_cell(value: str) -> str:
    """Escape a value for a Markdown table cell."""
    return value.replace("|", r"\|").replace("\n", " ")


def render_index(uses: list[FirstUse]) -> str:
    """Render the generated first-use index section."""
    lines = [
        INDEX_START,
        "## 首次英文标注索引",
        "",
        "本节由 `scripts/check_term_first_use.py --write` 按整书阅读顺序生成；",
        "它记录已经使用 `\\termfirst` 标注过的术语，不得手工编辑。",
        "",
        "| 英文原词 | 中文译名 | 首次标注位置 | 源码 |",
        "| --- | --- | --- | --- |",
    ]
    for use in uses:
        source = use.source.as_posix()
        lines.append(
            f"| {markdown_cell(use.english)} "
            f"| {markdown_cell(use.chinese)} "
            f"| {markdown_cell(use.location)} "
            f"| `{source}:{use.line}` |"
        )
    lines.extend([INDEX_END, ""])
    return "\n".join(lines)


def split_existing_index(text: str) -> tuple[str, str | None, str]:
    """Split terminology text around the generated index."""
    if INDEX_START not in text and INDEX_END not in text:
        return text.rstrip() + "\n\n", None, ""
    if text.count(INDEX_START) != 1 or text.count(INDEX_END) != 1:
        raise ValueError("term-first-use index markers must each occur once")
    before, remainder = text.split(INDEX_START, maxsplit=1)
    current_body, after = remainder.split(INDEX_END, maxsplit=1)
    current = INDEX_START + current_body + INDEX_END + "\n"
    return before.rstrip() + "\n\n", current, after.lstrip("\n")


def validate_manual_table(text: str) -> list[str]:
    """Reject obsolete manually maintained first-use fields and placeholders."""
    problems: list[str] = []
    if MANUAL_TABLE_HEADER not in text.splitlines():
        problems.append(
            "manual terminology table must contain exactly the four maintained "
            "fields: English, Chinese, status, and notes"
        )
    if "待回溯" in text:
        problems.append(
            "obsolete '待回溯' placeholder found in manual terminology table"
        )
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="rewrite the generated index after validating duplicate use",
    )
    args = parser.parse_args()

    uses, problems = scan_first_uses()
    if problems:
        for problem in problems:
            print(f"ERROR: {problem}")
        return 1

    text = TERMINOLOGY.read_text(encoding="utf-8")
    try:
        before, current, after = split_existing_index(text)
    except ValueError as error:
        print(f"ERROR: {error}")
        return 1
    manual_problems = validate_manual_table(before)
    if manual_problems:
        for problem in manual_problems:
            print(f"ERROR: {problem}")
        return 1
    expected = render_index(uses)

    if args.write:
        TERMINOLOGY.write_text(before + expected + after, encoding="utf-8")
        print(f"Wrote {len(uses)} first-use terms to {TERMINOLOGY.relative_to(ROOT)}.")
        return 0

    if current != expected:
        print(
            "ERROR: terminology first-use index is missing or stale; "
            "run scripts/check_term_first_use.py --write"
        )
        return 1

    print(f"Verified {len(uses)} unique first-use terminology annotations.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
