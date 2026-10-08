# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

The code in this file is for research/prototyping only.

## Response style
- Be concise. Answer yes/no questions with yes or no.
- Prefer responses in ASD-ste100

## Project

A Rust port of the Python package in `../gsearch` (shape search in large geometric graphs; read
`../gsearch/CLAUDE.md` for the algorithm and data format). The user is learning Rust by writing
the port themselves, following the instructional guide in `guide/`.

**The user writes the code in `src/`.** Do not implement slices in `src/` unless asked. Your
job is design help, explaining Rust vs Python differences, and writing guide slices.

## The guide

- `guide/README.md` is the roadmap. It has the slice table, the module map from Python to Rust,
  the design decisions (D1–D9, with status), the Python behaviour the port must keep, and a
  concept index that records where each Rust concept is first explained.
- Each slice is one file, `guide/slice-NN-*.md`. Only the next slices are written in full. When
  asked for the next slice, read the earlier slices and the concept index first, so that
  concepts are not explained twice. Point back to them instead ("Slice 1, Step 6"). Then add
  the new concepts to the index.
- Write slices by following @.claude/instructional_plan_best_practice.md.
- Before you put code or expected output in a slice, build it in a scratch Cargo project and
  run it, so that every "Expected:" line is real output.
- If the user changes a design decision, update its status in `guide/README.md`, then the
  slices that follow.

## Commands

```powershell
cargo build                 # debug build
cargo test                  # all tests
cargo test geometry         # tests whose path contains "geometry" (like pytest -k)
cargo test io::tests::parses_nodes_and_edges   # one test
cargo fmt; cargo clippy --all-targets          # before each commit
cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
cargo build --release       # always use release for timing
```

## Test data

The data is in `../gsearch/data/db/` (it is not in this repo). Small files for quick checks:
`queries/gd00000_q.json` (3 nodes) and `graphs/gg00041.json` (78 nodes, 114 edges).
`graphs/go00001.json` (238 MB, 452628 nodes) is the large-file timing check. `manifest.jsonl`
contains duplicate `graph_id` rows, so check the counts against the real file.
