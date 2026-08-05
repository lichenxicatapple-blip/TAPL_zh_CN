#!/usr/bin/env python3
"""Extract compiled Rust regions for inclusion in the LaTeX book."""

from __future__ import annotations

import argparse
import json
import re
import sys
import textwrap
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = ROOT / "code" / "book-snippets" / "src"
OUTPUT_ROOT = ROOT / "build" / "code-snippets"
TEX_ROOT = ROOT / "tex"

BEGIN = re.compile(r"^\s*// TAPL-SNIPPET-BEGIN: ([a-z0-9][a-z0-9-]*)\s*$")
END = re.compile(r"^\s*// TAPL-SNIPPET-END: ([a-z0-9][a-z0-9-]*)\s*$")
COUNTERPART_REFERENCE = re.compile(
    r"\\taplrustcounterpart(?:\[[^\]]*\])?"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)
SUPPORT_REFERENCE = re.compile(
    r"\\taplrustsupport(?:\[[^\]]*\])?"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)
OCAML_BEGIN = re.compile(r"\\begin\{taplocamlcode\}(?:\[[^\]]*\])?")
OCAML_END = re.compile(r"\\end\{taplocamlcode\}")
OCAML_WITH_RUST = re.compile(
    r"\\end\{taplocamlcode\}\s*"
    r"\\taplrustcounterpart(?:\[[^\]]*\])?"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)


def extract() -> dict[str, tuple[Path, str]]:
    snippets: dict[str, tuple[Path, str]] = {}
    for source in sorted(SOURCE_ROOT.rglob("*.rs")):
        active_name: str | None = None
        active_lines: list[str] = []
        for line_number, line in enumerate(
            source.read_text(encoding="utf-8").splitlines(), start=1
        ):
            begin = BEGIN.match(line)
            end = END.match(line)
            if begin:
                if active_name is not None:
                    raise ValueError(
                        f"{source}:{line_number}: nested snippet inside {active_name}"
                    )
                active_name = begin.group(1)
                active_lines = []
                continue
            if end:
                if active_name != end.group(1):
                    raise ValueError(
                        f"{source}:{line_number}: closing {end.group(1)} "
                        f"while {active_name or 'nothing'} is open"
                    )
                if active_name in snippets:
                    raise ValueError(f"duplicate snippet name: {active_name}")
                body = textwrap.dedent("\n".join(active_lines)).strip() + "\n"
                snippets[active_name] = (source.relative_to(ROOT), body)
                active_name = None
                active_lines = []
                continue
            if active_name is not None:
                active_lines.append(line)
        if active_name is not None:
            raise ValueError(f"{source}: unclosed snippet {active_name}")
    return snippets


def references() -> set[str]:
    names: list[str] = []
    errors: list[str] = []
    for source in sorted(TEX_ROOT.rglob("*.tex")):
        contents = source.read_text(encoding="utf-8")
        begin_count = len(OCAML_BEGIN.findall(contents))
        end_count = len(OCAML_END.findall(contents))
        paired = OCAML_WITH_RUST.findall(contents)
        counterparts_in_file = COUNTERPART_REFERENCE.findall(contents)
        support_in_file = SUPPORT_REFERENCE.findall(contents)
        if begin_count != end_count:
            errors.append(
                f"{source.relative_to(ROOT)}: {begin_count} OCaml starts, "
                f"{end_count} OCaml ends"
            )
        if end_count != len(paired):
            errors.append(
                f"{source.relative_to(ROOT)}: every OCaml block must be followed "
                "immediately by one Rust counterpart"
            )
        if len(counterparts_in_file) != len(paired):
            errors.append(
                f"{source.relative_to(ROOT)}: Rust counterpart found without an "
                "immediately preceding OCaml block"
            )
        names.extend(counterparts_in_file)
        names.extend(support_in_file)

    duplicates = sorted(name for name, count in Counter(names).items() if count > 1)
    if duplicates:
        errors.append(f"duplicate Rust snippet references: {', '.join(duplicates)}")
    if errors:
        raise ValueError("\n".join(errors))
    return set(names)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate markers and LaTeX references without writing files",
    )
    args = parser.parse_args()

    snippets = extract()
    referenced = references()
    missing = referenced - snippets.keys()
    unused = snippets.keys() - referenced
    if missing or unused:
        if missing:
            print(f"missing Rust snippets: {', '.join(sorted(missing))}", file=sys.stderr)
        if unused:
            print(f"unreferenced Rust snippets: {', '.join(sorted(unused))}", file=sys.stderr)
        return 1

    if args.check:
        print(f"Verified {len(snippets)} compiled Rust snippet references.")
        return 0

    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)
    for name, (_, body) in snippets.items():
        (OUTPUT_ROOT / f"{name}.rs").write_text(body, encoding="utf-8")
    manifest = {
        name: {"source": str(source), "output": f"{name}.rs"}
        for name, (source, _) in sorted(snippets.items())
    }
    (OUTPUT_ROOT / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Extracted {len(snippets)} compiled Rust snippets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
