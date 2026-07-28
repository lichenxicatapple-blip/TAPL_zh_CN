#!/usr/bin/env python3
"""Reproducibly extract the two raster reference figures from the TAPL preface."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from PIL import Image
from pypdf import PdfReader


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "source" / "split" / "manifest.json"
OUTPUT_DIR = ROOT / "figures" / "original" / "preface"
EXPECTED_OUTPUTS = {
    "chapter-dependencies.png": (
        "4410367f6fcda57b6f01098da177f9166f03ee7dee12c8cad72fa327c80392c7"
    ),
    "sample-syllabus-reference.png": (
        "18034554a47186b1871d18488f3ddc65d2d5aa133cb62f96cc6110f50e87d2e3"
    ),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def preface_entry() -> dict:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    matches = [
        item
        for item in manifest["segments"]
        if item["title"] == "Preface"
    ]
    if len(matches) != 1:
        raise SystemExit(f"Expected one Preface entry, found {len(matches)}")
    return matches[0]


def largest_image(reader: PdfReader, page_number: int) -> Image.Image:
    images = list(reader.pages[page_number - 1].images)
    if not images:
        raise SystemExit(f"No image found on local preface page {page_number}")
    image_file = max(
        images,
        key=lambda item: item.image.width * item.image.height,
    )
    return image_file.image.convert("RGB")


def main() -> None:
    entry = preface_entry()
    source = ROOT / entry["path"]
    actual_hash = sha256(source)
    if actual_hash != entry["sha256"]:
        raise SystemExit(
            f"Preface PDF checksum mismatch: expected {entry['sha256']}, "
            f"got {actual_hash}"
        )

    reader = PdfReader(source)
    if len(reader.pages) != entry["source_pdf_pages"]["count"]:
        raise SystemExit("Preface PDF page count does not match the manifest")

    outputs = {
        4: "chapter-dependencies.png",
        7: "sample-syllabus-reference.png",
    }
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for page_number, name in outputs.items():
        image = largest_image(reader, page_number)
        # The CHM conversion stores these two bitmaps bottom-to-top and flips
        # them when painting the PDF page. Normalize them for ordinary use.
        image = image.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
        output = OUTPUT_DIR / name
        image.save(output, format="PNG", optimize=True)
        output_hash = sha256(output)
        expected_hash = EXPECTED_OUTPUTS[name]
        if output_hash != expected_hash:
            raise SystemExit(
                f"{output.relative_to(ROOT)} checksum mismatch: "
                f"expected {expected_hash}, got {output_hash}"
            )
        print(
            f"{output.relative_to(ROOT)}: "
            f"{image.width}x{image.height}, sha256={output_hash}"
        )


if __name__ == "__main__":
    main()
