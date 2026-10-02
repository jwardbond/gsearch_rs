# gsearch in Rust: roadmap

This guide ports the Python package in `../gsearch` to Rust. You write the code. The guide
gives you the design decisions, explains what is different in Rust, and checks each step with
a command and its expected output.

The guide follows `.claude/instructional_plan_best_practice.md`. Each slice is one file and ends
in something you can run. Only the next slices are written in full. Ask Claude to write the next
slice when you get to it, so that it uses the decisions you made on the way.

## Your starting point

These answers shape how much each slice explains. If they change, tell Claude so it can adjust
the remaining slices.

- You know Python and numpy well. You use static type hints and a type checker.
- You understand references (a variable that points at data owned somewhere else).
- You are new to Rust. Ownership, borrowing, traits, enums, `Option`/`Result` and Cargo are
  all explained when you first use them.
- Stack vs heap and generics are new. They are explained when the code first needs them.
- Code style: "try first, then compare" for parts that look like Python. "Guided build" for
  parts that are specific to Rust.
- The goal is to learn Rust, get speed on the large graphs, and run in parallel. The end
  product is a standalone CLI first, with Python bindings (PyO3) possible as a last slice.

## Slices

| # | Slice | You can see | Status |
|---|-------|-------------|--------|
| 0 | [Toolchain and first program](slice-00-setup.md) | `cargo run` prints a greeting; you read your first compiler error | written |
| 1 | [Read a graph file](slice-01-read-json.md) | `gsearch <file>` prints `gd00000_q: 3 nodes, 3 edges`; a 238 MB graph loads in under a second | written |
| 2 | [Geometry](slice-02-geometry.md) | `Point`, `dist_to_segment`, `side_of`, with the ported pytest cases passing | written |
| 3 | [Graph type](slice-03-graph.md) | `Graph::from_file`, `distance_from_node`, `radius_from`; `cargo test graph` shows `19 passed`; a 238 MB graph builds in about 1 s. Port `test_core.py` | written |
| 4 | Spatial index and crop | `query_radius` and `make_crop` (connected component of the anchor). Port `test_spatial.py` | outline |
| 5 | Node matching | Annulus masks, handedness split, injective assignments, `get_candidate_matches`. Port `test_align_nodes.py` | outline |
| 6 | Procrustes alignment | Closed-form 2D fit (rotation or reflection). Port `TestAlignGraph` | outline |
| 7 | Edge routing | Dijkstra with a binary heap, `find_paths`, `align_and_score`. Port `TestFindPaths` | outline |
| 8 | Pipeline and CLI | `run_gsearch`, `clap` arguments, JSON results, progress bar, logging. First full search on a real graph, compared with Python | outline |
| 9 | Speed | Release profile, profiling, removing repeated allocations, timing on `gd`/`gg`/`go` graphs | outline |
| 10 | Parallel search | `rayon` over candidate anchors | outline |
| 11 | Python bindings (optional) | `maturin` + `PyO3`, so notebooks call the Rust search | outline |

## Module map

| Python (`../gsearch/src/gsearch`) | Rust (`src/`) | Slice |
|---|---|---|
| `cli.py` | `main.rs` (a thin binary) | 1, 8 |
| `io.py` | `io.rs` | 1 |
| `_euclidean_distance`, `_dist_to_segment`, `_cross_signs` | `geometry.rs` | 2 |
| `networkx.Graph` | `graph.rs` (your own type, or `petgraph`) | 3 |
| `spatial_graph_index.py` | `spatial.rs` | 4 |
| `align_nodes.py` | `align_nodes.rs` | 5 |
| `_align_graph` | `geometry.rs` or `procrustes.rs` | 6 |
| `align_edges.py` | `align_edges.rs` | 7 |
| `core.py` | `search.rs` | 8 |
| `log.py` | `log` + `env_logger` crates | 8 |
| `visualization.py` | Not ported. Notebooks read the Rust JSON output | 8 |

## Design decisions

Each decision has a status. "Decided" means the slices are written that way. You can still
change one; tell Claude, and the later slices are updated.

**D1. A library crate with a thin binary (decided, Slice 1).** Almost all code goes in
`src/lib.rs` and its modules. `src/main.rs` only reads arguments and calls the library, like
`cli.py`. Tests and the future Python bindings then use the library directly.

**D2. JSON with `serde` (decided, Slice 1).** The file format becomes Rust structs, and `serde`
writes the parsing code. We turn on the `float_roundtrip` feature so that floats are parsed
to the same bits as Python's `json`. Slice 1 shows why this matters.

**D3. Errors (decided for now, Slice 1).** Functions that can fail for reasons outside the
program (a missing file, bad JSON) return `Result`. For now the error type is
`Box<dyn Error>`, which holds "any error". Slice 3 adds one error type of our own,
`GraphError`, so that tests can check which error came back. Slice 8 may change this to the
`anyhow` and `thiserror` crates. Bugs (a broken invariant, a position out of range) panic.

**D4. Plain loops and a small `Point` type, no matrix crate (decided, Slice 2).** In Rust, a
loop compiles to fast machine code, so we do not need numpy-style vectorization to be fast.
Masks and distances become loops over slices. Procrustes in 2D has a closed form (Slice 6).

**D5. Graph storage (decided: own compact struct, written in Slice 3).** This choice shapes
Slices 3 to 7.
- *Own compact struct (chosen).* Node ids, coordinates and adjacency lists are plain
  `Vec`s indexed by position (`0..n`), plus a `HashMap` from file id to position. It uses
  little memory for 10^6 nodes and is easy to crop. You write Dijkstra and the
  connected-component search yourself (about 60 lines in total).
- *`petgraph` crate.* This is closer to networkx: Dijkstra and connected components are
  ready to use. But its generic types are harder to read in a first Rust project, and a
  crop copies more data.

**D6. Spatial index (open; decide in Slice 4).** The `kiddo` crate is a KD-tree like scipy's
`KDTree`. A uniform grid that you write yourself is a good alternative, because every query
in gsearch uses a fixed radius.

**D7. External ids vs internal indices (decided, Slice 3).** The JSON ids (`8018`, `51516`)
are only used at the edges of the program. Inside, every node is a `usize` index. This
replaces the many `list.index(...)` and `{n: i for i, n in enumerate(...)}` calls in the
Python code.

**D8. CLI with `clap` (decided, Slice 8).** The results are written as JSON, so the Python
notebooks can draw them.

**D9. Parallelism with `rayon` (decided, Slice 10).** Each candidate anchor is independent,
so the outer loop of `run_gsearch` becomes a parallel iterator.

### Behaviour to keep (from reading the Python code)

The Rust port should give the same results as Python. These details of the Python code
affect the results, so the later slices copy them:

- `query_radius` includes points exactly on the radius (`<=`), and sorts the results by
  distance.
- `make_crop` keeps only the connected component that contains the anchor.
- The anchor of `Q` is its first node in file order. In Rust this is position `0`.
- `Graph::from_file` is stricter than `nx.Graph` (Slice 3): a repeated node id and an edge to a
  missing node are errors, repeated edges count once, and self-loops are dropped. None of
  these occurs in the data.
- `radius_from` of a graph with one node is `0.0`. Python's `max` raises `ValueError` there.
  This is the only planned difference from Python.
- `get_candidate_matches` chooses the alignment node with the fewest annulus matches, and
  takes the first one if there is a tie.
- Nodes on the anchor-to-alignment axis go into both the rotation and the reflection mask.
- `_align_graph` allows reflections. It finds the best orthogonal matrix, not only the best
  rotation.
- The final score is `node_score + weight * edge_score`. Note that the docstring of
  `align_and_score` says `weight * node + (1 - weight) * edge`. The code and the docstring do
  not agree. The port follows the code unless you decide otherwise.
- The Python CLI does not print `results`. The Rust CLI will print them.

## Concept index

Each concept is explained once, where you first type it. Later slices point back here.

| Concept | Introduced in |
|---|---|
| `rustup`, `rustc`, `cargo` | Slice 0, Steps 1-2 |
| `fn main`, `println!`, macros | Slice 0, Step 6 |
| debug vs release builds | Slice 0, Step 7; Slice 1, Step 17 |
| `let` and `let mut`, reading compiler errors | Slice 0, Step 8 |
| `cargo fmt`, `cargo clippy` | Slice 0, Step 10 |
| crates, `cargo add`, features, `Cargo.lock` | Slice 1, Step 1 |
| library vs binary crate, `mod`, `pub`, `use` | Slice 1, Step 2 |
| structs, integer and float types, `String`, `Vec` | Slice 1, Step 3 |
| type aliases | Slice 1, Step 3 |
| `#[derive]`, `Debug`, `Deserialize` | Slice 1, Step 4 |
| `&str` and borrowing a function argument | Slice 1, Step 6 |
| `Result`, `todo!()` | Slice 1, Step 6 |
| unit tests, `#[cfg(test)]`, `assert_eq!`, raw strings, `unwrap` | Slice 1, Step 7 |
| implicit return (last expression) | Slice 1, Step 9 |
| `Option`, `expect`, panics | Slice 1, Step 11 |
| `?`, `Box<dyn Error>`, `Path`/`PathBuf` | Slice 1, Steps 13-14 |
| `Copy`, `Clone`, moves | Slice 2, Steps 2-3 |
| `impl` blocks, methods, `self`, associated functions | Slice 2, Step 4 |
| traits, operator overloading (`Sub`, `Add`, `Mul`) | Slice 2, Step 7 |
| iterators, `map`, `collect`, closures | Slice 2, Step 10 |
| `const`, no default arguments | Slice 2, Step 13 |
| enums, `match`, `Eq` vs `PartialEq` | Slice 2, Steps 13-14 |
| private fields, `crate::` paths, parallel `Vec`s | Slice 3, Step 2 |
| enums that hold data, `Display`, `impl Error`, `&mut` parameter | Slice 3, Step 3 |
| `&self` methods, tuples, `result.err()`, `to_string` | Slice 3, Step 4 |
| `Vec::with_capacity`, `vec!`, `HashMap`, `enumerate`, `Option::copied` | Slice 3, Step 6 |
| slices (`&[T]`), lifetime elision | Slice 3, Step 7 |
| closures that capture variables, `ok_or`, `?` with an own error, `sum::<T>()` | Slice 3, Step 8 |
| `&mut` loops, the borrowing rule (many readers or one writer), `sort_unstable`, `dedup` | Slice 3, Step 8 |
| a borrow cannot outlive its owner (`E0505`) | Slice 3, Step 9 |
| `#[should_panic]`, `all`, `windows`, `zip`, `find` | Slice 3, Step 10 |
| ranges, `filter`, `sort_by`, `total_cmp`, `f64` is not `Ord`, stable sort | Slice 3, Step 11 |
| `fold`, passing a function by name | Slice 3, Step 12 |
| `Instant`, `eprintln!` | Slice 3, Step 14 |
