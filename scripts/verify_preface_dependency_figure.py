#!/usr/bin/env python3
"""Verify the node and edge semantics of the redrawn Preface dependency figure."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "figures" / "redrawn" / "preface" / "chapter-dependencies.tex"

# Each tuple is (later chapter, prerequisite chapter, is_partial_dependency).
# The list was transcribed and cross-checked against Figure P-1 in the author's
# official frontmatter PDF (print page xvi).
EXPECTED_EDGES = {
    (3, 2, False),
    (4, 3, False),
    (5, 3, False),
    (8, 3, False),
    (7, 4, False),
    (6, 5, False),
    (9, 5, False),
    (7, 6, False),
    (10, 7, False),
    (9, 8, False),
    (10, 9, False),
    (11, 9, False),
    (12, 9, False),
    (20, 9, False),
    (23, 9, False),
    (17, 10, False),
    (25, 10, False),
    (13, 11, False),
    (14, 11, False),
    (15, 11, False),
    (22, 11, False),
    (18, 13, False),
    (16, 15, False),
    (18, 15, False),
    (19, 15, False),
    (20, 15, True),
    (21, 15, False),
    (26, 15, False),
    (17, 16, False),
    (28, 16, False),
    (27, 18, False),
    (21, 20, False),
    (29, 23, False),
    (24, 23, False),
    (26, 23, False),
    (25, 24, False),
    (26, 24, True),
    (27, 26, False),
    (28, 26, False),
    (31, 26, False),
    (32, 27, False),
    (30, 28, True),
    (30, 29, False),
    (31, 30, False),
    (32, 31, False),
}


def main() -> int:
    text = SOURCE.read_text(encoding="utf-8")

    nodes = [
        int(number)
        for number in re.findall(
            r"\\node\[chapter\]\s+\(c(\d+)\)", text
        )
    ]
    expected_nodes = set(range(1, 33))
    actual_nodes = set(nodes)

    edge_blocks = re.findall(
        r"\\draw\[(dependency|partial dependency)\](.*?);",
        text,
        flags=re.DOTALL,
    )
    edges: list[tuple[int, int, bool]] = []
    for style, block in edge_blocks:
        endpoints = [
            int(number) for number in re.findall(r"\(c(\d+)\)", block)
        ]
        if len(endpoints) != 2:
            raise ValueError(
                f"dependency edge must have two endpoints: {style!r} {endpoints!r}"
            )
        edges.append(
            (
                endpoints[0],
                endpoints[1],
                style == "partial dependency",
            )
        )

    actual_edges = set(edges)
    problems: list[str] = []
    if len(nodes) != len(actual_nodes):
        problems.append("duplicate chapter node declarations")
    if actual_nodes != expected_nodes:
        problems.append(
            f"node mismatch: missing={sorted(expected_nodes - actual_nodes)}, "
            f"extra={sorted(actual_nodes - expected_nodes)}"
        )
    if len(edges) != len(actual_edges):
        problems.append("duplicate dependency edge declarations")
    if actual_edges != EXPECTED_EDGES:
        problems.append(
            f"edge mismatch: missing={sorted(EXPECTED_EDGES - actual_edges)}, "
            f"extra={sorted(actual_edges - EXPECTED_EDGES)}"
        )

    if problems:
        for problem in problems:
            print(f"ERROR: {problem}")
        return 1

    solid = sum(not partial for _, _, partial in edges)
    partial = sum(partial for _, _, partial in edges)
    print(
        f"Verified dependency figure: {len(nodes)} nodes, "
        f"{solid} solid edges, {partial} partial edges."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
