# Slice 8: The pipeline and the CLI

**Goal.** Port `core.py` (`run_gsearch`) and `cli.py`. The search goes over every candidate anchor
in `G`, crops `G` around it, finds the mappings (Slice 5), aligns and scores each one (Slices 6
and 7), and keeps the `k` best alignments. A command-line program (decision D8) wraps it and can
write the results to a JSON file.

**Outcome.** `cargo test search::` reports `9 passed`. This command prints one summary line, and
writes a JSON file:

```powershell
cargo run --release -- ..\gsearch\data\db\queries\go00038_q.json ..\gsearch\data\db\graphs\go00038.json -k 3 -o out.json
```

On all six small graphs of Step 9, the number of mappings is the same as in Python.

**New ideas.** `clap` and its derive; optional arguments (`Option<PathBuf>`); a struct that
holds references and its lifetime parameter (`Report<'a>`); the `log` macros and `env_logger`;
a progress bar that the library does not know about (a closure that changes a variable, `FnMut`);
`Serialize` and `serde_json::to_writer`; `partition_point`, `Vec::insert` and `truncate`
(a sorted list that stays short); `.into()` to turn a `&str` into an error; `BufWriter`.

**Starting point.** Slice 7 is done: `cargo test` reports `0 failed` and `16 ignored`.

---

## Read this before you run anything on a real graph

The number of mappings is the problem, not the speed of the code. `get_candidate_matches`
returns all the mappings of an anchor, and with the default tolerance (`10`) the road graph
`gr00049` (1128 nodes) has **281 132 804 mappings in total**. My run took 168 s in release mode
with the placeholders. With a real `find_paths` (one Dijkstra for each query edge for each
mapping) the same run would take hours. The count grows very fast with the tolerance:

| `-e` | mappings in `gr00049` | time (release, placeholders) |
|---|---|---|
| 6 | 408 | 0.006 s |
| 7 | 12 234 | 0.013 s |
| 8 | 21 400 | 0.022 s |
| 9 | 8 498 802 | 4.7 s |
| 10 | 281 132 804 | 167.6 s |

(The times are from my machine. Your times will differ. The counts are the ones from Slice 5,
Step 10, if your alignment node rule is the one of Slice 5, Step 5.)

So this slice uses the small graphs of `..\gsearch\data\db` for checks. Do not run `gr00047` or
the large `gd`, `gg` and `go` graphs. A module that ranks the anchors first (your plan) is how
this count will come down later, and `get_candidate_anchors` in Step 3 is the place for it.

---

## The design

### What the Python code does

`run_gsearch(Q, G, tol, k)`:

1. Index `G` (`SpatialGraphIndex`).
2. The anchor of `Q` is its first node. `Q_radius` is the largest distance from the anchor to
   another node of `Q`.
3. `_get_candidate_anchors` scores every node of `G` and sorts them, best first. The score is
   `1.0` for all nodes (`_similarity_score` is a stub), so the order is the order of the nodes.
4. For each candidate anchor: crop `G` with radius `Q_radius + tol`, get the candidate matches,
   align and score each one, and keep the result unless it is `None`.
5. Sort all the alignments by score and return the first `k`.

### What changes in Rust

| Python | Rust |
|---|---|
| `alignments = []`, then `sorted(...)[:k]` | `TopK`: a list that never holds more than `k` items |
| `tqdm` | `indicatif` in `main.rs`; the library calls a closure after each anchor |
| `print(...)` in `run_gsearch` | `log` macros in the library, `println!` in `main.rs` |
| `click` | `clap` |
| the CLI prints nothing about the results | the CLI prints a summary, and `-o` writes the results |
| `--log-file` | not ported: use `2> run.log` in the shell |

**Only `k` alignments stay in memory.** Python keeps every alignment, and sorts at the end. A
search with millions of mappings would hold millions of alignments (each has several lists).
`TopK` inserts each new alignment at its place in a sorted list of at most `k` items. For equal
scores it keeps the older item first, as Python's stable `sorted` does.

**The library does not draw a progress bar.** Slice 1 (D1) puts the logic in the library and the
user interface in `main.rs`. `run_gsearch` takes a closure `on_anchor(done, total)` and calls it
after each anchor. `main.rs` gives it a closure that moves the bar. A test gives it a closure
that records the calls.

**The output file (decided with you).** `-o PATH` writes one JSON object:

```text
{ "query", "graph", "tol", "k", "weight", "n_anchors", "n_mappings",
  "placeholder_scoring",
  "results": [ { "score", "transform": {"r", "t"}, "mapping": [[q_id, g_id], ...],
                 "nodes": [ids], "edges": [[id, id], ...] } ] }
```

Ids are the ids in the files (D7). Without `-o`, no file is written, and the summary goes to
standard output. `placeholder_scoring` is `true` while `fit` or `find_paths` is a placeholder, and
the program also prints a warning.

---

## Steps

### 1. Dependencies

- [ ] Run:

  ```powershell
  cargo add clap --features derive
  cargo add indicatif log env_logger
  ```

  Slice 1, Step 1 explains crates and features. `clap` is the argument parser (`click` in
  Python), and its `derive` feature lets you describe the arguments with a struct.
  `indicatif` is `tqdm`. `log` is the `logging` module: it gives the macros `debug!`, `info!`
  and so on, and does nothing by itself. `env_logger` is a **backend** that prints the messages.
  The split lets a library write log messages without choosing where they go.

- [ ] Run `cargo build`. Expected: it downloads and builds the new crates, and finishes without
  an error.

### 2. The types of the search

- [ ] Create `src/search.rs`, and add `pub mod search;` to `src/lib.rs`:

  ```rust
  //! The search: every candidate anchor, every mapping, the best alignments (`core.py` in Python).

  use log::{debug, info};

  use crate::alignment::{Alignment, align_and_score};
  use crate::graph::Graph;
  use crate::match_generation::get_candidate_matches;
  use crate::spatial_index::SpatialIndex;

  /// The default weight of the edge score. The Python CLI does not have an option for it.
  pub const DEFAULT_WEIGHT: f64 = 0.5;

  pub struct SearchParams {
      pub tol: f64,
      pub k: usize,
      pub weight: f64,
  }

  pub struct SearchOutcome {
      /// At most `k` alignments, best first.
      pub alignments: Vec<Alignment>,
      pub n_anchors: usize,
      pub n_mappings: usize,
  }
  ```

  `SearchParams` replaces a list of three loose arguments. When you add a fourth setting later,
  no call in the program changes except the places that need the new one. The `use` lines that
  the later steps need are already here. The compiler warns about unused imports until Step 5.

### 3. The candidate anchors

Python's `_get_candidate_anchors(anchor_id, Q, G)` returns `(node, score)` pairs, best first. All
scores are `1.0`. Keep the function and its signature: it is the **seam** where your ranking
module goes later (the caller does not change).

**Try first.** Write `fn get_candidate_anchors(_q_anchor: usize, _q: &Graph, g: &Graph) -> Vec<(usize, f64)>`.
It gives `(position, 1.0)` for every node of `g`, and sorts the list by score, best first. A
sort in Rust is stable (Slice 3, Step 11), as in Python, so equal scores keep the order of the
nodes.

- [ ] Add a stub, and a test module at the end of the file:

  ```rust
  /// The nodes of `g` to try as the anchor, with a score, best first.
  ///
  /// For now every node has the score `1.0`, as in Python, so the order is the order of the nodes.
  /// A module that ranks the candidates replaces the body of this function.
  fn get_candidate_anchors(_q_anchor: usize, _q: &Graph, g: &Graph) -> Vec<(usize, f64)> {
      todo!()
  }

  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::io::NodeId;
      use crate::test_fixtures::file;

      fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
          Graph::from_file(&file(nodes, edges)).unwrap()
      }

      #[test]
      fn every_node_is_a_candidate_anchor() {
          let q = graph(&[(1, 0.0, 0.0)], &[]);
          let g = graph(&[(10, 0.0, 0.0), (11, 1.0, 0.0), (12, 2.0, 0.0)], &[]);

          let anchors = get_candidate_anchors(0, &q, &g);

          // all scores are 1.0, so the order of the nodes is kept
          assert_eq!(anchors, vec![(0, 1.0), (1, 1.0), (2, 1.0)]);
      }
  }
  ```

- [ ] Run `cargo test every_node`. Expected: the test fails with `not yet implemented`.
- [ ] Replace the body:

  ```rust
      let mut anchors: Vec<(usize, f64)> = (0..g.node_count()).map(|j| (j, 1.0)).collect();
      anchors.sort_by(|a, b| b.1.total_cmp(&a.1));
      anchors
  ```

  1. `(0..g.node_count()).map(|j| (j, 1.0))` makes the pairs. `1.0` is an `f64` because of the
     return type.
  2. `b.1.total_cmp(&a.1)` compares the scores with the arguments in the reverse order, so the
     highest score comes first (Slice 3, Step 11 explains `sort_by` and `total_cmp`).
  3. The arguments with a `_` in the signature are not used now. They are in the signature
     because a ranking module needs them (Slice 6, Step 5 explains the `_`).

- [ ] Run `cargo test every_node`. Expected: `1 passed`.

### 4. `TopK`

**Try first.** A struct with a `k` and a `Vec<Alignment>` called `items`, with `new(k)` and
`push(alignment)`. `push` must keep `items` sorted by score and never longer than `k`. Equal
scores keep their order of arrival.

- [ ] Add after `get_candidate_anchors`, before `mod tests`:

  ```rust
  /// The `k` lowest scores seen so far, in order. It never holds more than `k` alignments.
  struct TopK {
      k: usize,
      items: Vec<Alignment>,
  }

  impl TopK {
      fn new(k: usize) -> TopK {
          TopK {
              k,
              items: Vec::with_capacity(k),
          }
      }

      /// Alignments with equal scores keep the order in which they came, as in Python's `sorted`.
      fn push(&mut self, alignment: Alignment) {
          todo!()
      }
  }
  ```

- [ ] Add five helpers and tests to `mod tests` (after the first test). The `alignment` helper
  makes a small alignment with a given score and a tag (a node id) that identifies it:

  ```rust
      use crate::procrustes::Transform;

      fn alignment(score: f64, tag: NodeId) -> Alignment {
          Alignment {
              score,
              transform: Transform::IDENTITY,
              mapping: vec![(tag, tag)],
              nodes: vec![tag],
              edges: Vec::new(),
          }
      }

      fn scores(top: &TopK) -> Vec<f64> {
          top.items.iter().map(|a| a.score).collect()
      }

      #[test]
      fn top_k_keeps_the_lowest_scores_in_order() {
          let mut top = TopK::new(2);

          for score in [5.0, 1.0, 3.0, 2.0] {
              top.push(alignment(score, 0));
          }

          assert_eq!(scores(&top), vec![1.0, 2.0]);
      }

      #[test]
      fn top_k_keeps_the_first_of_equal_scores() {
          let mut top = TopK::new(2);

          for tag in [1, 2, 3] {
              top.push(alignment(1.0, tag));
          }

          // the first two stay, as with a stable sort
          let tags: Vec<NodeId> = top.items.iter().map(|a| a.nodes[0]).collect();
          assert_eq!(tags, vec![1, 2]);
      }

      #[test]
      fn top_k_with_k_zero_keeps_nothing() {
          let mut top = TopK::new(0);

          top.push(alignment(1.0, 0));

          assert!(top.items.is_empty());
      }

      #[test]
      fn top_k_with_fewer_than_k_keeps_all() {
          let mut top = TopK::new(5);

          top.push(alignment(2.0, 0));
          top.push(alignment(1.0, 0));

          assert_eq!(scores(&top), vec![1.0, 2.0]);
      }
  ```

  The `use` line goes at the top of `mod tests`, with the other `use` lines. Put the helpers
  before the tests that use them, or after: Rust does not care about the order of functions.

- [ ] Run `cargo test top_k`. Expected: `0 passed; 4 failed`, each with `not yet implemented`.
- [ ] Replace the body of `push`:

  ```rust
          let at = self.items.partition_point(|a| a.score <= alignment.score);
          if at < self.k {
              self.items.insert(at, alignment);
              self.items.truncate(self.k);
          }
  ```

  1. `partition_point(pred)` works on a list that is already split in two: first all the items
     for which `pred` is true, then all the items for which it is false. It returns the index of
     the split, with a binary search. Here the split is "scores that are `<=` the new score",
     then "scores that are larger". The new item goes right after the items with an equal score,
     and that keeps the older items first.
  2. If `at` is `k` or more, the new item would be the `(k+1)`th or later, so it is dropped.
     This also handles `k == 0`: `at` is `0`, and `0 < 0` is false, so nothing is kept.
  3. `insert(at, item)` shifts the later items by one place. For a small `k` this is cheap, and
     `truncate(k)` drops the item that was pushed out. `Vec::truncate` does nothing if the list is
     already short enough.
  4. `push` takes `alignment` by value and moves it into the list (Slice 2, Step 3). The test
     before the `if` only reads `alignment.score`.

- [ ] Run `cargo test top_k`. Expected: `4 passed`.

### 5. `run_gsearch`

**Try first.** Write the function with this signature:

```rust
pub fn run_gsearch(
    q: &Graph,
    index: &SpatialIndex,
    params: &SearchParams,
    mut on_anchor: impl FnMut(usize, usize),
) -> SearchOutcome
```

Steps: the anchor of `q` is position `0`; the crop radius is `q.radius_from(0) + tol`; get the
candidate anchors; for each one, crop, get the mappings, align and score each, push to a
`TopK`; call `on_anchor(done, total)` after each anchor. Count the anchors and the mappings.

Write the tests first. They use a small search that you can check by hand.

- [ ] Add these to `mod tests`:

  ```rust
      /// A 3-4-5 triangle as Q, and a G with one copy of it, far away, and one stray node.
      fn q_and_g() -> (Graph, Graph) {
          let q = graph(
              &[(1, 0.0, 0.0), (2, 3.0, 0.0), (3, 0.0, 4.0)],
              &[(1, 2), (1, 3)],
          );
          let g = graph(
              &[
                  (11, 100.0, 100.0),
                  (12, 103.0, 100.0),
                  (13, 100.0, 104.0),
                  (14, 300.0, 300.0),
              ],
              &[(11, 12), (11, 13)],
          );
          (q, g)
      }

      #[test]
      fn finds_the_copy_and_counts_the_work() {
          let (q, g) = q_and_g();
          let index = SpatialIndex::new(g);
          let params = SearchParams {
              tol: 0.5,
              k: 3,
              weight: DEFAULT_WEIGHT,
          };

          let outcome = run_gsearch(&q, &index, &params, |_, _| {});

          // 4 anchors are tried, and only the anchor 11 has a mapping: Q 1->11, 2->12, 3->13
          assert_eq!(outcome.n_anchors, 4);
          assert_eq!(outcome.n_mappings, 1);
          assert_eq!(outcome.alignments.len(), 1);
          assert_eq!(
              outcome.alignments[0].mapping,
              vec![(1, 11), (2, 12), (3, 13)]
          );
      }

      #[test]
      fn reports_progress_after_each_anchor() {
          let (q, g) = q_and_g();
          let index = SpatialIndex::new(g);
          let params = SearchParams {
              tol: 0.5,
              k: 1,
              weight: DEFAULT_WEIGHT,
          };
          let mut calls = Vec::new();

          run_gsearch(&q, &index, &params, |done, total| calls.push((done, total)));

          assert_eq!(calls, vec![(1, 4), (2, 4), (3, 4), (4, 4)]);
      }

      #[test]
      fn k_limits_the_alignments() {
          let (q, g) = q_and_g();
          let index = SpatialIndex::new(g);
          let params = SearchParams {
              tol: 0.5,
              k: 0,
              weight: DEFAULT_WEIGHT,
          };

          let outcome = run_gsearch(&q, &index, &params, |_, _| {});

          // the mapping is counted, but nothing is kept
          assert_eq!(outcome.n_mappings, 1);
          assert!(outcome.alignments.is_empty());
      }

      #[test]
      fn a_query_that_is_not_in_g_gives_nothing() {
          let (_, g) = q_and_g();
          let q = graph(&[(1, 0.0, 0.0), (2, 50.0, 0.0)], &[(1, 2)]);
          let index = SpatialIndex::new(g);
          let params = SearchParams {
              tol: 0.5,
              k: 1,
              weight: DEFAULT_WEIGHT,
          };

          let outcome = run_gsearch(&q, &index, &params, |_, _| {});

          assert_eq!(outcome.n_mappings, 0);
          assert!(outcome.alignments.is_empty());
      }
  ```

  1. `q_and_g` has one copy of Q (nodes `11`, `12`, `13`) and one node far away. Only the
     anchor `11` has a crop in which Q fits. In the crop of `12`, the node `13` is at distance
     5, and Q needs a node at distance 4 (within 0.5).
  2. `|_, _| {}` is a closure that takes two arguments and does nothing. In the progress test
     the closure changes `calls`, so `calls` must be `mut`, and the closure borrows it for the
     call. This is why `on_anchor` is `FnMut` and not `Fn` (Slice 5, Step 7 explains
     `Fn`, `FnMut` and `FnOnce`).

- [ ] Add a stub before `mod tests`, and run `cargo test search::`. Expected: the four new tests
  fail with `not yet implemented`, and the others pass.

  ```rust
  /// Searches `index.graph()` for `q`. `on_anchor(done, total)` is called after each anchor.
  ///
  /// The anchor of `q` is its first node, and `q` must have at least one node.
  pub fn run_gsearch(
      q: &Graph,
      index: &SpatialIndex,
      params: &SearchParams,
      mut on_anchor: impl FnMut(usize, usize),
  ) -> SearchOutcome {
      todo!()
  }
  ```

**Suggested code.** The head of the function:

```rust
    let q_anchor = 0;
    let radius = q.radius_from(q_anchor) + params.tol;

    let anchors = get_candidate_anchors(q_anchor, q, index.graph());
    let total = anchors.len();

    let mut best = TopK::new(params.k);
    let mut n_mappings = 0;
```

The loop:

```rust
    for (done, &(g_anchor, _)) in anchors.iter().enumerate() {
        let crop = index.make_crop(g_anchor, radius);

        // the anchor is position 0 of a crop
        if let Some(mappings) = get_candidate_matches(q, &crop, q_anchor, 0, params.tol) {
            debug!("anchor {g_anchor}: {} mappings", mappings.len());
            n_mappings += mappings.len();
            for mapping in &mappings {
                if let Some(alignment) =
                    align_and_score(q, &crop, mapping, params.tol, params.weight)
                {
                    best.push(alignment);
                }
            }
        }
        on_anchor(done + 1, total);
    }
```

And the end:

```rust
    info!(
        "{total} anchors, {n_mappings} mappings, {} kept",
        best.items.len()
    );
    SearchOutcome {
        alignments: best.items,
        n_anchors: total,
        n_mappings,
    }
```

**Explanation.**

1. `Q_radius + tol` is the radius that Python uses for the crop. The crop keeps only the
   connected part around the anchor, and puts the anchor at position `0` (Slice 4, `make_crop`).
   So the anchor argument for `G` is always `0`, and the anchor of `Q` is `q_anchor`, which is
   also `0`. The two zeros mean different things, which is why `q_anchor` has a name.
2. `for (done, &(g_anchor, _)) in anchors.iter().enumerate()`: `enumerate` gives `(index, item)`,
   and the item is a `&(usize, f64)`. The pattern `&(g_anchor, _)` takes the pair apart, copies
   the position, and ignores the score. `done` counts from `0`, so `on_anchor` gets `done + 1`.
3. `if let Some(mappings) = ...` runs the block only for `Some`. For `None` (no mapping for this
   anchor) nothing happens, and the loop goes on. It is `if matches is not None:`.
4. `for mapping in &mappings` borrows each mapping (a `&Vec<usize>`), and `align_and_score` takes
   a `&[usize]`. The `&Vec` turns into a `&[usize]` at the call.
5. `if let Some(alignment) = ...` drops the `None` results (a mapping with no path). This is the
   `if result is None: continue` of Python. The `best.push(alignment)` moves the alignment into
   `TopK`.
6. `debug!` and `info!` take a format string like `println!`, and a variable in the string
   can be written as `{g_anchor}`. They print nothing until `main.rs` starts a logger (Step 7).
   `debug!` is for details (one line for each anchor), and `info!` is the summary.
7. The `crop` and the `mappings` of one anchor are dropped at the end of each loop step, so
   only one anchor's mappings are in memory at a time. Python's `tqdm` calls are not here: this
   is why `on_anchor` exists.
8. `SearchOutcome { alignments: best.items, ... }` moves the list out of `best`. After that line
   `best` cannot be used, and the compiler checks it.

- [ ] Replace the `todo!()` with the three pieces, in order.
- [ ] Run `cargo test search::`. Expected: `9 passed; 0 failed`.

### 6. Make the result writable

`serde` writes JSON from a struct that has `#[derive(Serialize)]`. It is the reverse of
`Deserialize` (Slice 1, Step 4). Every type inside the struct must also be `Serialize`.

- [ ] In `src/geometry.rs`, add `use serde::Serialize;` at the top, and change the derive of
  `Point`:

  ```rust
  #[derive(Debug, Copy, Clone, PartialEq, Serialize)]
  pub struct Point {
  ```

- [ ] In `src/procrustes.rs`, add `use serde::Serialize;` before the `use crate::geometry::Point;`
  line, and change the derive of `Transform` to `#[derive(Debug, Clone, Copy, PartialEq, Serialize)]`.
- [ ] In `src/alignment.rs`, add `use serde::Serialize;` after `use std::collections::BTreeSet;`
  (with an empty line between them), and change the derive of `Alignment` to
  `#[derive(Debug, Clone, PartialEq, Serialize)]`.
- [ ] Run `cargo build`. Expected: no error.

  `Point` becomes `{"x": ..., "y": ...}`, a `[[f64; 2]; 2]` becomes a list of lists, a tuple
  `(NodeId, NodeId)` becomes a list of two numbers, and a `Vec` becomes a list.

### 7. `main.rs`

Replace the whole of `src/main.rs` in three pieces. The Slice 5 `main` is not needed any more.

**The arguments.** With `clap`, a struct describes the command line. A doc comment on a field
becomes its help text.

- [ ] Replace `src/main.rs`:

  ```rust
  use std::error::Error;
  use std::path::PathBuf;

  use clap::Parser;

  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;
  use gsearch::search::DEFAULT_WEIGHT;
  use gsearch::spatial_index::SpatialIndex;

  /// Search a graph for the places that match a query shape.
  #[derive(Parser)]
  #[command(version)]
  struct Args {
      /// The query graph (JSON).
      query: PathBuf,
      /// The graph to search (JSON).
      graph: PathBuf,
      /// The tolerance, in the units of the graph.
      #[arg(short = 'e', long, default_value_t = 10.0)]
      tol: f64,
      /// How many alignments to keep.
      #[arg(short, default_value_t = 1)]
      k: usize,
      /// The weight of the edge score in the score.
      #[arg(short, long, default_value_t = DEFAULT_WEIGHT)]
      weight: f64,
      /// Write the results to this file (JSON).
      #[arg(short, long)]
      output: Option<PathBuf>,
      /// Log the work of every anchor.
      #[arg(short, long)]
      verbose: bool,
  }

  fn main() -> Result<(), Box<dyn Error>> {
      let args = Args::parse();

      let level = if args.verbose { "debug" } else { "info" };
      env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).init();

      let query_file = read_graph_file(&args.query)?;
      let query = Graph::from_file(&query_file)?;
      if query.node_count() == 0 {
          return Err("the query has no nodes".into());
      }
      let graph_file = read_graph_file(&args.graph)?;
      let index = SpatialIndex::new(Graph::from_file(&graph_file)?);
      println!(
          "{}: {} nodes. {}: {} nodes.",
          query_file.graph_id,
          query.node_count(),
          graph_file.graph_id,
          index.graph().node_count(),
      );
      Ok(())
  }
  ```

  1. `#[derive(Parser)]` writes the code that reads `std::env::args()` and fills the struct. It
     is the `@click.command()` and `@click.option` of `cli.py`, and the struct is the list of
     parameters. `Args::parse()` also handles `--help`, `--version` and error messages, and exits
     the program when the arguments are wrong.
  2. A field without `#[arg]` (`query`, `graph`) is a positional argument. `PathBuf` is the owned
     form of `Path` (Slice 1, Step 14), and `clap` turns the text into one.
  3. `#[arg(short = 'e', long, ...)]` gives the option `-e` and `--tol`. `long` alone uses the
     field name. `short` alone uses its first letter, so `k` gives `-k` and `verbose` gives `-v`.
     The Python CLI used `-e` for the tolerance, so this one does the same.
  4. `default_value_t = 10.0` is the default, and `_t` means "a value of the field type", so
     the help text shows it. `DEFAULT_WEIGHT` is a `const`, so it can be used here.
  5. `Option<PathBuf>` makes `-o` optional: `None` when the user does not give it. This is the
     same `Option` as everywhere else. No special "optional" feature is needed.
  6. `#[command(version)]` adds `--version`, with the version from `Cargo.toml`.
  7. `env_logger::Builder::from_env(...)` starts the logger. It reads the variable `RUST_LOG`
     (for example `RUST_LOG=debug`) and uses `default_filter_or(level)` when the variable is not
     set. `-v` chooses `debug`. Messages go to standard error, so they do not mix with the
     summary on standard output.
  8. `return Err("the query has no nodes".into())`: `.into()` turns a `&str` into the
     `Box<dyn Error>` of the return type. `Graph::radius_from(0)` would panic on an empty graph,
     and Python raises an error there. D3 says that a bad input is an error, and a broken
     invariant is a panic. An empty query file is a bad input.
  9. `Graph::from_file(&query_file)?` returns a `GraphError`. The `?` turns it into the
     `Box<dyn Error>` (Slice 3, Step 3).

- [ ] Run:

  ```powershell
  cargo run --release -- ..\gsearch\data\db\queries\gg00041_q.json ..\gsearch\data\db\graphs\gg00041.json
  ```

  Expected: `gg00041_q: 5 nodes. gg00041: 78 nodes.`

- [ ] Run `cargo run --release -- --help`. Expected:

  ```text
  Search a graph for the places that match a query shape

  Usage: gsearch.exe [OPTIONS] <QUERY> <GRAPH>

  Arguments:
    <QUERY>  The query graph (JSON)
    <GRAPH>  The graph to search (JSON)

  Options:
    -e, --tol <TOL>        The tolerance, in the units of the graph [default: 10]
    -k <K>                 How many alignments to keep [default: 1]
    -w, --weight <WEIGHT>  The weight of the edge score in the score [default: 0.5]
    -o, --output <OUTPUT>  Write the results to this file (JSON)
    -v, --verbose          Log the work of every anchor
    -h, --help             Print help
    -V, --version          Print version
  ```

  The help text is made from your doc comments. (The final full stops are removed.)

- [ ] Run `cargo run --release` with no arguments. Expected: an error that says
  `the following required arguments were not provided: <QUERY> <GRAPH>`, and the usage line.

**The search.** Now call `run_gsearch` with a progress bar.

- [ ] Replace the `use` lines at the top of `main.rs` with:

  ```rust
  use std::error::Error;
  use std::path::PathBuf;
  use std::time::Instant;

  use clap::Parser;
  use indicatif::ProgressBar;

  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;
  use gsearch::search::{DEFAULT_WEIGHT, SearchParams, run_gsearch};
  use gsearch::spatial_index::SpatialIndex;
  use gsearch::{path_scoring, procrustes};
  ```

- [ ] Replace the `println!` and the `Ok(())` at the end of `main` with:

  ```rust
      let params = SearchParams {
          tol: args.tol,
          k: args.k,
          weight: args.weight,
      };
      let bar = ProgressBar::new(0);
      let start = Instant::now();
      let outcome = run_gsearch(&query, &index, &params, |done, total| {
          bar.set_length(total as u64);
          bar.set_position(done as u64);
      });
      bar.finish_and_clear();

      let placeholder_scoring = procrustes::IS_PLACEHOLDER || path_scoring::IS_PLACEHOLDER;
      println!(
          "{} in {}: {} anchors, {} mappings, {} alignments kept ({:.3} s)",
          query_file.graph_id,
          graph_file.graph_id,
          outcome.n_anchors,
          outcome.n_mappings,
          outcome.alignments.len(),
          start.elapsed().as_secs_f64(),
      );
      if let Some(best) = outcome.alignments.first() {
          println!("lowest score: {}", best.score);
      }
      if placeholder_scoring {
          println!("warning: the scores use a placeholder (see procrustes.rs, path_scoring.rs)");
      }
      Ok(())
  ```

  1. The closure `|done, total| { bar.set_length(...); bar.set_position(...); }` uses the
     variable `bar` from the code around it, and `run_gsearch` calls it after each anchor. The
     bar draws on standard error, and hides itself when the output is not a terminal. The
     bar does not know the total at the start (the library knows it), so the closure sets the
     length on each call. This costs nothing.
  2. `total as u64`: `as` changes the number type (Slice 4, Step 7). `indicatif` uses `u64`.
  3. `bar.finish_and_clear()` removes the bar, so that the summary is the last thing on the
     screen.
  4. The two modules' `IS_PLACEHOLDER` constants (Slices 6 and 7) decide the warning. When you
     write `fit` and `find_paths` and set both to `false`, the warning goes away. You do not
     change `main.rs`.
  5. `outcome.alignments.first()` is an `Option<&Alignment>`: `None` for an empty list. It is
     the best alignment, because `TopK` is sorted. This is `alignments[0] if alignments else None`.
  6. `{:.3}` prints a float with three decimals (Slice 3, Step 14).

- [ ] Run:

  ```powershell
  cargo run --release -- ..\gsearch\data\db\queries\go00038_q.json ..\gsearch\data\db\graphs\go00038.json -k 3
  ```

  Expected (the time differs; the other numbers are the same):

  ```text
  [... INFO  gsearch::search] 100 anchors, 4 mappings, 3 kept
  go00038_q in go00038: 100 anchors, 4 mappings, 3 alignments kept (0.013 s)
  lowest score: 3689.6519817600556
  warning: the scores use a placeholder (see procrustes.rs, path_scoring.rs)
  ```

  The first line is the log message on standard error, with a time stamp. The score is the
  placeholder's: with the identity transform, it is the size of the shift between the frame of
  `Q` and the frame of `G`. Four mappings were found, and all four were kept (`-k 3` keeps
  three).

- [ ] Run the same with `-v`. Expected: before the `INFO` line, one `DEBUG gsearch::search] anchor N: M mappings`
  line for each anchor that has a mapping. (`gg00041` shows one line, `anchor 71: 1 mappings`.)

**The output file.**

- [ ] Replace the `use` lines at the top of `main.rs` with:

  ```rust
  use std::error::Error;
  use std::fs::File;
  use std::io::BufWriter;
  use std::path::PathBuf;
  use std::time::Instant;

  use clap::Parser;
  use indicatif::ProgressBar;
  use serde::Serialize;

  use gsearch::alignment::Alignment;
  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;
  use gsearch::search::{DEFAULT_WEIGHT, SearchParams, run_gsearch};
  use gsearch::spatial_index::SpatialIndex;
  use gsearch::{path_scoring, procrustes};
  ```

- [ ] Add the struct before `main`:

  ```rust
  /// What `-o` writes.
  #[derive(Serialize)]
  struct Report<'a> {
      query: &'a str,
      graph: &'a str,
      tol: f64,
      k: usize,
      weight: f64,
      n_anchors: usize,
      n_mappings: usize,
      /// True while the transform or the routing is a placeholder: the scores mean nothing then.
      placeholder_scoring: bool,
      results: &'a [Alignment],
  }
  ```

  A struct that holds a **reference** needs a **lifetime parameter**: the `'a`. It says "the
  data that this struct points at lives at least as long as the struct". Here `query` points
  into `query_file`, and `results` points into `outcome`. Both are alive until the end of `main`,
  longer than `report`, so it is correct, and the compiler checks it (Slice 3, Step 9: a borrow
  cannot outlive its owner). The other choice is `String` and `Vec<Alignment>`: it works without
  `'a`, but it copies the strings and all the alignments to write them.

- [ ] Add before the last `Ok(())`:

  ```rust
      if let Some(path) = &args.output {
          let report = Report {
              query: &query_file.graph_id,
              graph: &graph_file.graph_id,
              tol: args.tol,
              k: args.k,
              weight: args.weight,
              n_anchors: outcome.n_anchors,
              n_mappings: outcome.n_mappings,
              placeholder_scoring,
              results: &outcome.alignments,
          };
          serde_json::to_writer(BufWriter::new(File::create(path)?), &report)?;
      }
  ```

  1. `if let Some(path) = &args.output`: borrow the `Option` and take the path out of it only
     if it is `Some`.
  2. `File::create(path)?` opens the file for writing and replaces an old file. It gives an
     `io::Error` that `?` turns into the `Box<dyn Error>`.
  3. `BufWriter` collects the many small writes of `serde_json` and writes them in blocks.
     Without it, each small piece is a call to the operating system.
  4. `to_writer` writes compact JSON in one line. `to_writer_pretty` adds line breaks and
     indents, but puts every number of a `Vec` on its own line, and a 20-node mapping then takes
     60 lines. Python reads either one.
  5. `&query_file.graph_id` is a `&String`, and the field wants a `&str`. The `&String` turns
     into a `&str` by itself (Slice 1, Step 6).

- [ ] Run:

  ```powershell
  cargo run --release -- ..\gsearch\data\db\queries\gg00041_q.json ..\gsearch\data\db\graphs\gg00041.json -o out.json
  python -c "import json; d = json.load(open('out.json')); print({k: v for k, v in d.items() if k != 'results'}); print(d['results'][0])"
  ```

  Expected, for the first line:

  ```text
  {'query': 'gg00041_q', 'graph': 'gg00041', 'tol': 10.0, 'k': 1, 'weight': 0.5, 'n_anchors': 78, 'n_mappings': 1, 'placeholder_scoring': True}
  ```

  and the result is a dict with the keys `score`, `transform`, `mapping`, `nodes` and `edges`.
  In `mapping`, the ids on the two sides are equal, because this query was cut out of its own
  graph: `[[71, 71], [49, 49], [50, 50], [57, 57], [58, 58]]`. `edges` is empty, because the
  placeholder `find_paths` gives no path. `nodes` is `[49, 50, 57, 58, 71]`.

- [ ] Run with a file that does not exist: `cargo run --release -- nofile.json x.json`. Expected:
  `Error: Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }`.
  The program ends with a non-zero code. This is the `Debug` form of the error that `main` prints
  when it returns an `Err`. D3 said Slice 8 may change this to `anyhow`. It is not worth it yet:
  there is one `main`, and the message is clear enough.

### 8. Look at it: how the tolerance changes the work

- [ ] Run, from the repo folder, with the road graph from Slice 5:

  ```powershell
  cargo run --release -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json -e 8
  ```

  Expected: `1128 anchors, 21400 mappings, 1 alignments kept`, in about `0.02 s`.

- [ ] Run it with `-e 9`. Expected: `8498802 mappings`, in a few seconds. This is the last one
  to try. `-e 10` is the 168 s run, and with real scoring it is out of reach.

  The mappings of one anchor are all made before any is scored, so the memory is the number of
  mappings of the **largest** anchor, and not the total. Keep Task Manager open if you try `-e 9`.

### 9. Compare with Python

The number of mappings does not depend on the order of the nodes, so Python and Rust must agree.
From the `gsearch` folder:

```powershell
cd ..\gsearch
$check = @'
import sys
from gsearch.align_nodes import get_candidate_matches
from gsearch.core import _get_radius
from gsearch.io import load_graph
from gsearch.spatial_graph_index import SpatialGraphIndex

q, g = load_graph(sys.argv[1]), load_graph(sys.argv[2])
q_anchor = next(iter(q))
radius = _get_radius(q, q_anchor) + 10.0
index = SpatialGraphIndex(g)
total = sum(
    len(get_candidate_matches(q, index.make_crop(n, radius), q_anchor, n, 10.0)) for n in g
)
print(sys.argv[2].split("\\")[-1], "mappings:", total)
'@
foreach ($id in "gg00041", "gd00040", "go00038", "go00028", "gg00016", "gd00032") {
    $check | uv run python - data\db\queries\${id}_q.json data\db\graphs\$id.json
}
cd ..\gsearch_rs
```

- [ ] Run the same six graphs with the Rust program (default tolerance). Expected, for both:

  | graph | mappings |
  |---|---|
  | `gg00041` | 1 |
  | `gd00040` | 2 |
  | `go00038` | 4 |
  | `go00028` | 2 |
  | `gg00016` | 1 |
  | `gd00032` | 1 |

  These are the numbers that I got from both programs. If yours differ, the first place to look
  is the alignment node rule (Slice 5, Step 5: the **fewest** matches, the first one on a tie),
  and then a distance difference of exactly `tol` (the test is `<=`).

**When you replace the placeholders.** Each of these six queries was cut out of its own graph,
so after your `fit` and your `find_paths` work, the best alignment must be an exact copy: the
`lowest score` must be below `1e-6`, and the warning must be gone. With a version of my own for
both (a reference, not in your repo), `lowest score` was `0` for `gg00041` and between `1e-11` and
`1e-9` for the other five, and the Python scores for the three graphs that I tried were the same size. Some mappings have no path
(for example 4 mappings give 2 alignments for `go00038`), so `alignments kept` can be smaller
than `mappings`.

### 10. Format, lint, commit

- [ ] Run `cargo fmt; cargo clippy --all-targets`. Expected: no warning from the new files.
- [ ] Run `cargo test`. Expected: `0 failed`.
- [ ] Commit:

  ```powershell
  git add src Cargo.toml Cargo.lock guide GLOSSARY.md
  git commit -m "feat: search pipeline and CLI with JSON output"
  ```

---

## What comes next

Slice 9 is about finding where the time goes. The profile of this program on `gr00049` (with `-e 9`)
is a good first target: it has millions of mappings, and every one goes through
`align_and_score`.
