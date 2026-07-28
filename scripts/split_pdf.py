#!/usr/bin/env python3
"""Split the local TAPL conversion into reviewable, page-exact units."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
from typing import Any

from pypdf import PdfReader, PdfWriter


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "Benjamin_C._Pierce-Types_and_Programming_Languages-The_MIT_Press(2002).pdf"
OUTPUT_ROOT = ROOT / "source" / "split"
MANIFEST = OUTPUT_ROOT / "manifest.json"
EXPECTED_SOURCE_SHA256 = "065390fa14bc289975935ee3aa6bf9f3b937f01f27c4d2fc02858355a6924560"
EXPECTED_SOURCE_PAGES = 625


def segment(
    path: str,
    kind: str,
    title: str,
    pdf_start: int,
    pdf_end: int,
    print_pages: str,
    anchor: str,
) -> dict[str, Any]:
    return {
        "path": f"source/split/{path}",
        "kind": kind,
        "title": title,
        "pdf_start": pdf_start,
        "pdf_end": pdf_end,
        "print_pages": print_pages,
        "anchor": anchor,
    }


SEGMENTS: list[dict[str, Any]] = [
    segment(
        "frontmatter/00-web-table-of-contents_p001-p002.pdf",
        "frontmatter",
        "CHM conversion table of contents",
        1,
        2,
        "not applicable",
        "Table of Contents",
    ),
    segment(
        "frontmatter/01-back-cover_p003.pdf",
        "frontmatter",
        "Back cover",
        3,
        3,
        "not applicable",
        "Back Cover",
    ),
    segment(
        "frontmatter/02-title-and-copyright_p004.pdf",
        "frontmatter",
        "Title and copyright",
        4,
        4,
        "front matter; conversion is not page-for-page",
        "Types and Programming Languages",
    ),
    segment(
        "frontmatter/03-preface_p005-p016.pdf",
        "frontmatter",
        "Preface",
        5,
        16,
        "xiii-xxi",
        "Preface",
    ),
    segment(
        "parts/part-01-untyped-systems_p033.pdf",
        "part",
        "Part I: Untyped Systems",
        33,
        33,
        "21-22",
        "Part I: Untyped Systems",
    ),
    segment(
        "parts/part-02-simple-types_p088.pdf",
        "part",
        "Part II: Simple Types",
        88,
        88,
        "89-90",
        "Part II: Simple Types",
    ),
    segment(
        "parts/part-03-subtyping_p174.pdf",
        "part",
        "Part III: Subtyping",
        174,
        174,
        "179-180",
        "Part III: Subtyping",
    ),
    segment(
        "parts/part-04-recursive-types_p254.pdf",
        "part",
        "Part IV: Recursive Types",
        254,
        254,
        "265-266",
        "Part IV: Recursive Types",
    ),
    segment(
        "parts/part-05-polymorphism_p301.pdf",
        "part",
        "Part V: Polymorphism",
        301,
        301,
        "315-316",
        "Part V: Polymorphism",
    ),
    segment(
        "parts/part-06-higher-order-systems_p408.pdf",
        "part",
        "Part VI: Higher-Order Systems",
        408,
        408,
        "437-438",
        "Part VI: Higher-Order Systems",
    ),
    segment(
        "parts/part-07-appendices_p454.pdf",
        "part",
        "Appendices divider (the conversion labels this Part VII)",
        454,
        454,
        "491-492",
        "Part VII: Appendices",
    ),
    segment(
        "appendices/appendix-a-solutions_p455-p524.pdf",
        "appendix",
        "Appendix A: Solutions to Selected Exercises",
        455,
        524,
        "493-564",
        "Appendix A:",
    ),
    segment(
        "appendices/appendix-b-notation_p525-p526.pdf",
        "appendix",
        "Appendix B: Notational Conventions",
        525,
        526,
        "565-566",
        "Appendix B:",
    ),
    segment(
        "backmatter/references_p527-p569.pdf",
        "backmatter",
        "References",
        527,
        569,
        "567-604",
        "References",
    ),
    segment(
        "backmatter/index_p570-p621.pdf",
        "backmatter",
        "Index",
        570,
        621,
        "605-623",
        "Index",
    ),
    segment(
        "backmatter/list-of-figures_p622-p625.pdf",
        "backmatter",
        "List of Figures (conversion-generated)",
        622,
        625,
        "not present as a numbered section in the official contents",
        "List of Figures",
    ),
]


CHAPTERS = [
    (1, "introduction", "Introduction", 17, 25, "1-14"),
    (2, "mathematical-preliminaries", "Mathematical Preliminaries", 26, 32, "15-20"),
    (3, "untyped-arithmetic-expressions", "Untyped Arithmetic Expressions", 34, 51, "23-44"),
    (4, "ml-arithmetic-expressions", "An ML Implementation of Arithmetic Expressions", 52, 57, "45-50"),
    (5, "untyped-lambda-calculus", "The Untyped Lambda-Calculus", 58, 76, "51-74"),
    (6, "nameless-representation", "Nameless Representation of Terms", 77, 82, "75-82"),
    (7, "ml-lambda-calculus", "An ML Implementation of the Lambda-Calculus", 83, 87, "83-88"),
    (8, "typed-arithmetic-expressions", "Typed Arithmetic Expressions", 89, 97, "91-98"),
    (9, "simply-typed-lambda-calculus", "Simply Typed Lambda-Calculus", 98, 112, "99-112"),
    (10, "ml-simple-types", "An ML Implementation of Simple Types", 113, 116, "113-116"),
    (11, "simple-extensions", "Simple Extensions", 117, 143, "117-148"),
    (12, "normalization", "Normalization", 144, 148, "149-152"),
    (13, "references", "References", 149, 165, "153-170"),
    (14, "exceptions", "Exceptions", 166, 173, "171-178"),
    (15, "subtyping", "Subtyping", 175, 199, "181-208"),
    (16, "metatheory-of-subtyping", "Metatheory of Subtyping", 200, 211, "209-220"),
    (17, "ml-subtyping", "An ML Implementation of Subtyping", 212, 215, "221-224"),
    (18, "imperative-objects", "Case Study: Imperative Objects", 216, 238, "225-246"),
    (19, "featherweight-java", "Case Study: Featherweight Java", 239, 253, "247-264"),
    (20, "recursive-types", "Recursive Types", 255, 266, "267-280"),
    (21, "metatheory-of-recursive-types", "Metatheory of Recursive Types", 267, 300, "281-314"),
    (22, "type-reconstruction", "Type Reconstruction", 302, 320, "317-338"),
    (23, "universal-types", "Universal Types", 321, 343, "339-362"),
    (24, "existential-types", "Existential Types", 344, 357, "363-380"),
    (25, "ml-system-f", "An ML Implementation of System F", 358, 365, "381-388"),
    (26, "bounded-quantification", "Bounded Quantification", 366, 383, "389-410"),
    (27, "imperative-objects-redux", "Case Study: Imperative Objects, Redux", 384, 388, "411-416"),
    (28, "metatheory-of-bounded-quantification", "Metatheory of Bounded Quantification", 389, 407, "417-436"),
    (29, "type-operators-and-kinding", "Type Operators and Kinding", 409, 416, "439-448"),
    (30, "higher-order-polymorphism", "Higher-Order Polymorphism", 417, 430, "449-466"),
    (31, "higher-order-subtyping", "Higher-Order Subtyping", 431, 437, "467-474"),
    (32, "purely-functional-objects", "Case Study: Purely Functional Objects", 438, 453, "475-490"),
]

for number, slug, title, start, end, print_pages in CHAPTERS:
    SEGMENTS.append(
        segment(
            f"chapters/ch{number:02d}-{slug}_p{start:03d}-p{end:03d}.pdf",
            "chapter",
            f"Chapter {number}: {title}",
            start,
            end,
            print_pages,
            f"Chapter {number}:",
        )
    )

SEGMENTS.sort(key=lambda item: item["pdf_start"])


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def normalize_text(text: str) -> str:
    """Collapse conversion-induced whitespace without weakening title checks."""
    return " ".join(text.split())


def validate_source(reader: PdfReader) -> None:
    actual_sha256 = sha256(SOURCE)
    if actual_sha256 != EXPECTED_SOURCE_SHA256:
        raise RuntimeError(
            "Source PDF checksum changed: "
            f"expected {EXPECTED_SOURCE_SHA256}, got {actual_sha256}"
        )
    if len(reader.pages) != EXPECTED_SOURCE_PAGES:
        raise RuntimeError(
            f"Source page count changed: expected {EXPECTED_SOURCE_PAGES}, "
            f"got {len(reader.pages)}"
        )

    covered: list[int] = []
    for item in SEGMENTS:
        start = item["pdf_start"]
        end = item["pdf_end"]
        if start < 1 or end < start:
            raise RuntimeError(f"Invalid range in {item['path']}: {start}-{end}")
        covered.extend(range(start, end + 1))

        first_page_text = normalize_text(
            reader.pages[start - 1].extract_text() or ""
        )
        if item["anchor"] not in first_page_text:
            raise RuntimeError(
                f"Boundary check failed for {item['path']}: "
                f"{item['anchor']!r} not found on source PDF page {start}"
            )

    expected = list(range(1, EXPECTED_SOURCE_PAGES + 1))
    if covered != expected:
        missing = sorted(set(expected) - set(covered))
        repeated = sorted(page for page in set(covered) if covered.count(page) > 1)
        raise RuntimeError(
            f"Ranges do not cover the source exactly once; "
            f"missing={missing}, repeated={repeated}"
        )


def split(reader: PdfReader) -> None:
    manifest_entries: list[dict[str, Any]] = []

    for item in SEGMENTS:
        output_path = ROOT / item["path"]
        output_path.parent.mkdir(parents=True, exist_ok=True)
        temporary_path = output_path.with_suffix(".pdf.tmp")

        writer = PdfWriter()
        for page_number in range(item["pdf_start"], item["pdf_end"] + 1):
            writer.add_page(reader.pages[page_number - 1])
        writer.add_metadata(
            {
                "/Title": item["title"],
                "/Author": "Benjamin C. Pierce",
                "/Subject": "TAPL source segment for the Chinese translation project",
                "/SourceSHA256": EXPECTED_SOURCE_SHA256,
            }
        )
        writer.add_outline_item(item["title"], 0)

        with temporary_path.open("wb") as stream:
            writer.write(stream)
        os.replace(temporary_path, output_path)

        written = PdfReader(output_path)
        expected_pages = item["pdf_end"] - item["pdf_start"] + 1
        if len(written.pages) != expected_pages:
            raise RuntimeError(
                f"Written page count mismatch for {item['path']}: "
                f"expected {expected_pages}, got {len(written.pages)}"
            )

        manifest_entries.append(
            {
                "path": item["path"],
                "kind": item["kind"],
                "title": item["title"],
                "source_pdf_pages": {
                    "start": item["pdf_start"],
                    "end": item["pdf_end"],
                    "count": expected_pages,
                },
                "original_print_pages": item["print_pages"],
                "sha256": sha256(output_path),
            }
        )

    manifest = {
        "source": {
            "path": SOURCE.name,
            "sha256": EXPECTED_SOURCE_SHA256,
            "pdf_pages": EXPECTED_SOURCE_PAGES,
            "bibliographic_identity": {
                "title": "Types and Programming Languages",
                "author": "Benjamin C. Pierce",
                "publisher": "The MIT Press",
                "year": 2002,
                "isbn_10": "0-262-16209-1",
                "isbn_13": "978-0-262-16209-8",
            },
        },
        "page_numbering_note": (
            "The local file is a CHM-to-PDF conversion. Its PDF pagination is not "
            "page-for-page with the original printed edition."
        ),
        "segments": manifest_entries,
    }
    MANIFEST.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def verify_existing(reader: PdfReader) -> None:
    if not MANIFEST.exists():
        raise RuntimeError(f"Manifest not found: {MANIFEST}")
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    manifest_entries = {item["path"]: item for item in manifest["segments"]}

    for item in SEGMENTS:
        output_path = ROOT / item["path"]
        if not output_path.exists():
            raise RuntimeError(f"Split file missing: {output_path}")
        written = PdfReader(output_path)
        expected_pages = item["pdf_end"] - item["pdf_start"] + 1
        if len(written.pages) != expected_pages:
            raise RuntimeError(
                f"Page count mismatch for {item['path']}: "
                f"expected {expected_pages}, got {len(written.pages)}"
            )
        expected_hash = manifest_entries[item["path"]]["sha256"]
        actual_hash = sha256(output_path)
        if actual_hash != expected_hash:
            raise RuntimeError(
                f"Checksum mismatch for {item['path']}: "
                f"expected {expected_hash}, got {actual_hash}"
            )

        first_page_text = normalize_text(written.pages[0].extract_text() or "")
        if item["anchor"] not in first_page_text:
            raise RuntimeError(
                f"Output boundary check failed for {item['path']}: "
                f"{item['anchor']!r} not found"
            )

    print(
        f"Verified {len(SEGMENTS)} split PDFs covering all "
        f"{len(reader.pages)} source pages exactly once."
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Verify existing split files and their manifest without rewriting them.",
    )
    args = parser.parse_args()

    reader = PdfReader(SOURCE)
    validate_source(reader)
    if args.verify_only:
        verify_existing(reader)
    else:
        split(reader)
        verify_existing(reader)


if __name__ == "__main__":
    main()
