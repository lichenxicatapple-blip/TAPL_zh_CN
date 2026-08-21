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
END = re.compile(
    r"^(?P<prefix>.*?)// TAPL-SNIPPET-END: "
    r"(?P<name>[a-z0-9][a-z0-9-]*)\s*$"
)
COUNTERPART_REFERENCE = re.compile(
    r"\\taplrustcounterpart(?:\[[^\]]*\])?"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)
COUNTERPART_CHUNK_FIRST_REFERENCE = re.compile(
    r"\\taplrustcounterpartflowchunkfirst"
    r"\{([a-z0-9][a-z0-9-]*)\}\{\d+\}\{\d+\}"
)
COUNTERPART_CHUNK_REFERENCE = re.compile(
    r"\\taplrustcounterpartflowchunk(?:first)?"
    r"\{([a-z0-9][a-z0-9-]*)\}\{\d+\}\{\d+\}"
)
SUPPORT_REFERENCE = re.compile(
    r"\\taplrustsupport(?:\[[^\]]*\])?"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)
SUPPORT_CHUNK_REFERENCE = re.compile(
    r"\\taplrustsupport(?:flow)?chunk(?:first(?:compact)?)?"
    r"\{([a-z0-9][a-z0-9-]*)\}\{\d+\}\{\d+\}"
)
EXPLANATION_REFERENCE = re.compile(r"\\taplrustexplanation\{")
OCAML_SYNTAX_ONLY_REFERENCE = re.compile(r"(?<!\{)\\taplocamlsyntaxonly\b")
OCAML_BEGIN = re.compile(r"\\begin\{taplocamlcode\}(?:\[[^\]]*\])?")
OCAML_END = re.compile(r"\\end\{taplocamlcode\}")
OCAML_WITH_RUST = re.compile(
    r"\\end\{taplocamlcode\}\s*"
    r"\\(?:taplrustcounterpart(?:\[[^\]]*\])?|"
    r"taplrustcounterpartflowchunkfirst)"
    r"\{([a-z0-9][a-z0-9-]*)\}(?:\{\d+\}\{\d+\})?"
)
OCAML_WITH_RUST_EXPLANATION = re.compile(
    r"\\end\{taplocamlcode\}"
    r"(?:(?!\\begin\{taplocamlcode\}).)*?"
    r"\\taplrustexplanation\{",
    re.DOTALL,
)
OCAML_WITH_SYNTAX_ONLY = re.compile(
    r"\\end\{taplocamlcode\}\s*\\taplocamlsyntaxonly\b"
)
GROUPED_COUNTERPART_REFERENCE = re.compile(
    r"\\taplocamlgroupedcounterpart"
    r"\{([a-z0-9][a-z0-9-]*)\}"
)
OCAML_WITH_GROUPED_COUNTERPART = re.compile(
    r"\\end\{taplocamlcode\}\s*"
    r"\\taplocamlgroupedcounterpart"
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
                closing_name = end.group("name")
                if active_name != closing_name:
                    raise ValueError(
                        f"{source}:{line_number}: closing {closing_name} "
                        f"while {active_name or 'nothing'} is open"
                    )
                prefix = end.group("prefix").rstrip()
                if prefix:
                    active_lines.append(prefix)
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
    direct_names: list[str] = []
    chunk_names: list[str] = []
    errors: list[str] = []
    for source in sorted(TEX_ROOT.rglob("*.tex")):
        contents = source.read_text(encoding="utf-8")
        begin_count = len(OCAML_BEGIN.findall(contents))
        end_count = len(OCAML_END.findall(contents))
        paired = OCAML_WITH_RUST.findall(contents)
        explained = OCAML_WITH_RUST_EXPLANATION.findall(contents)
        syntax_only = OCAML_WITH_SYNTAX_ONLY.findall(contents)
        grouped = OCAML_WITH_GROUPED_COUNTERPART.findall(contents)
        counterparts_in_file = COUNTERPART_REFERENCE.findall(contents)
        counterpart_chunk_first_in_file = (
            COUNTERPART_CHUNK_FIRST_REFERENCE.findall(contents)
        )
        counterpart_chunks_in_file = COUNTERPART_CHUNK_REFERENCE.findall(contents)
        grouped_in_file = GROUPED_COUNTERPART_REFERENCE.findall(contents)
        support_in_file = SUPPORT_REFERENCE.findall(contents)
        support_chunks_in_file = SUPPORT_CHUNK_REFERENCE.findall(contents)
        explanations_in_file = EXPLANATION_REFERENCE.findall(contents)
        syntax_only_in_file = OCAML_SYNTAX_ONLY_REFERENCE.findall(contents)
        if begin_count != end_count:
            errors.append(
                f"{source.relative_to(ROOT)}: {begin_count} OCaml starts, "
                f"{end_count} OCaml ends"
            )
        if end_count != len(paired) + len(explained) + len(syntax_only) + len(grouped):
            errors.append(
                f"{source.relative_to(ROOT)}: every OCaml block must have either "
                "an immediate Rust counterpart, an explicit Rust explanation "
                "before the next OCaml block, an immediate OCaml-syntax-only marker, "
                "or a named grouped counterpart"
            )
        if Counter(counterparts_in_file + counterpart_chunk_first_in_file) != Counter(
            paired + grouped
        ):
            errors.append(
                f"{source.relative_to(ROOT)}: Rust counterparts do not match the "
                "immediate and grouped OCaml counterpart declarations"
            )
        if len(explanations_in_file) != len(explained):
            errors.append(
                f"{source.relative_to(ROOT)}: Rust explanation could not be associated "
                "with the preceding OCaml block"
            )
        if len(syntax_only_in_file) != len(syntax_only):
            errors.append(
                f"{source.relative_to(ROOT)}: OCaml-syntax-only marker found without "
                "an immediately preceding OCaml block"
            )
        if len(grouped_in_file) != len(grouped):
            errors.append(
                f"{source.relative_to(ROOT)}: grouped Rust counterpart marker found "
                "without an immediately preceding OCaml block"
            )
        names.extend(counterparts_in_file)
        names.extend(counterpart_chunks_in_file)
        names.extend(support_in_file)
        names.extend(support_chunks_in_file)
        direct_names.extend(counterparts_in_file)
        chunk_names.extend(counterpart_chunks_in_file)
        direct_names.extend(support_in_file)
        chunk_names.extend(support_chunks_in_file)

    duplicates = sorted(name for name, count in Counter(direct_names).items() if count > 1)
    if duplicates:
        errors.append(f"duplicate Rust snippet references: {', '.join(duplicates)}")
    mixed = sorted(set(direct_names) & set(chunk_names))
    if mixed:
        errors.append(
            "Rust snippets cannot be referenced both whole and in chunks: "
            + ", ".join(mixed)
        )
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
