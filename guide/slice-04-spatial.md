# Slice 4: Spatial index and crop

**Goal.** Port `spatial_graph_index.py`. A `SpatialIndex` owns a `Graph` and a KD-tree over its
coordinates (decision D6: the `kiddo` crate). `query_radius` finds every node within a
distance of a point, nearest first. `make_crop` cuts out the part of the graph around an
anchor: the nodes inside the radius that are connected to the anchor. The crop is a new, small
`Graph`, so this slice also adds two `Graph` methods: `subgraph` and `connected_component`.

**Outcome.** `cargo test spatial` reports `13 passed`, and this works:

```text
> cargo run --release -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json
gr00049_q: 10 nodes, crop radius 68.0499
load and index:  0.001 s
gr00049: 1128 nodes, 1205 edges
crop around node 0: 5 nodes, 4 edges
crop every node: 0.007 s
mean crop size: 12.86 nodes
```

The Python code makes the same crops, with the same mean size. On the 100 110-node Munich
graph, a crop around every node takes about 2 seconds in Rust and about 150 seconds in Python.

**New ideas.** A test module that other modules share; collecting into a `HashMap`;
`filter_map`; generic types and const generic parameters; arrays (`[f64; 2]`); a query
builder; `as` casts between number types; a struct that owns another struct; `vec![x; n]`;
`while let`; a graph search with a stack; `--nocapture`; reading arguments with `next()`.

**Starting point.** Slice 3 is done. `Graph::from_file`, `coord`, `neighbors`, `index_of` and
`radius_from` work, and `cargo test graph` reports `19 passed`.

Two notes about your files:

- The guide calls the adjacency field of `Graph` `adjacency`. Your `src/graph.rs` calls it
  `adj`. Use your name where Step 3 uses the field.
- The data of the earlier slices (`..\gsearch\data\db`) is not on this computer. This slice
  uses the road graphs in this repo, `data\roads\graphs` and `data\roads\queries`. Each graph
  has a query with the same number.

---

## The design

### What the Python code does

`SpatialGraphIndex.__init__` copies the graph and builds a scipy `KDTree` over the
coordinates. Row `i` of the tree is the `i`-th node of the graph, and `node_ids[i]` turns a
row back into an id.

`query_radius(point, radius)` asks the tree for every row within `radius` of `point`. scipy
returns the rows in index order, so the function sorts them by distance. It returns two lists:
the distances and the node ids.

`make_crop(node_id, radius)` calls `query_radius` around the anchor node, takes the subgraph
on those nodes, and then keeps only the connected component that contains the anchor.
`run_gsearch` calls it once for every candidate anchor, with the radius `Q_radius + tol`.

### The Rust version

| Python | Rust |
|---|---|
| `SpatialGraphIndex(graph)` copies the graph | `SpatialIndex::new(graph)` takes ownership of the graph |
| KD-tree row `i`, `node_ids[i]` | Tree item `i` is graph position `i`, so no list is needed |
| `query_radius` returns `(dists, ids)` | `query_radius` returns `Vec<(usize, f64)>`, like `distance_from_node` |
| `make_crop` returns an `nx.Graph` | `make_crop` returns a new `Graph` |
| `G.subgraph(nodes)` | `Graph::subgraph(&keep)` |
| `nx.node_connected_component(G, n)` | `Graph::connected_component(start)`, which returns a `Graph` |

Three choices need an explanation.

**The index owns the graph.** Python copies the graph so that later changes to the original
cannot break the index. In Rust, `SpatialIndex::new(graph)` moves the graph into the index
(Slice 2, Step 3). The caller then cannot use the original, so nothing can change it. There
is also no copy to pay for. Other code reads the graph through `index.graph()`.

**The component search is a `Graph` method.** A connected component has nothing to do with
coordinates, so it goes in `graph.rs`, not in the index. `make_crop` has two steps:

1. Crop to the radius: `subgraph` of the nodes that `query_radius` finds. Call this graph the
   circle.
2. Take the component of the anchor in the circle: `circle.connected_component(start)`.

The order matters. A path from the anchor can leave the circle and come back in. For
example, `A-B-C` is a path, `A` and `C` are inside the radius, and `B` is outside it. In the
full graph, `C` is in the component of `A`. In the circle, the edges to `B` are gone, so `C`
is not. The crop must not contain `C`. Python does the same two steps.

**The anchor is position 0 of the crop. No other order is promised.** `connected_component`
puts its start node first, just as the anchor of `Q` is position `0` of `Q` (Slice 3,
Step 13). The order of the other nodes has no meaning. In Python it comes from a `set`. The
same is true for ties in `query_radius`: kiddo, like `np.argsort`, can return two nodes at the
same distance in either order, and the tests do not depend on it. The crop keeps the NodeIds,
so a result in the crop can always be turned back into the ids of `G`.

### The tests

The Python tests are in `tests/test_spatial.py`, on the same six-node graph as Slice 3. Each
test goes to the module that owns the behaviour. A test of edges or coordinates in a crop is
a test of `subgraph`, so it goes in `graph.rs`:

| Python test | Rust test |
|---|---|
| `test_init_copies_graph` | Not a test. The compiler stops the use of a moved graph (experiment in Step 14) |
| `test_node_ids_align_with_kdtree_rows` | `every_node_finds_itself` |
| `test_query_radius_distances_align_with_ids` | `query_radius_distances_match_coordinates` |
| `test_query_radius_sorts_by_distance` | `query_radius_sorts_by_distance` |
| `test_query_radius_returns_exactly_nodes_within_radius` | `query_radius_returns_exactly_nodes_within_radius` |
| `test_query_radius_boundary_is_inclusive` | `query_radius_boundary_is_inclusive` |
| `test_query_radius_at_node_coords_includes_that_node` | `query_radius_at_node_coords_includes_that_node` |
| `test_query_radius_returns_coincident_nodes` | `query_radius_returns_coincident_nodes` |
| `test_query_radius_no_hits_returns_two_empty_lists` | `query_radius_with_no_hits_is_empty` |
| `test_make_crop_nodes` | `crop_has_the_nodes_within_the_radius` |
| (new) | `crop_starts_with_the_anchor` |
| `test_make_crop_edges_are_induced` | `subgraph_keeps_edges_between_kept_nodes` (`graph.rs`) |
| `test_make_crop_preserves_node_attributes` | `subgraph_follows_the_order_of_keep` (`graph.rs`) |
| `test_make_crop_is_unaffected_by_later_source_edits` | Not needed. `Graph` has no method that changes it |
| `test_make_crop_edits_do_not_affect_source` | Not needed, for the same reason |
| `test_make_crop_radius_zero` | `crop_with_radius_zero_is_the_anchor` |
| `test_make_crop_radius_zero_keeps_coincident_nodes` | `crop_with_radius_zero_keeps_coincident_nodes` |
| `test_make_crop_keeps_only_anchor_component` | `crop_drops_nodes_connected_only_outside_the_radius`, and the three `component_` tests (`graph.rs`) |

The two "edits" tests protect against a Python crop that is a view of the big graph. In Rust,
every `Graph` owns its own `Vec`s, and after `from_file` or `subgraph` no method can change
them. Neither problem can happen.

---

## Steps

### 1. Share the test fixtures

The tests of `spatial_index.rs` need the same six-node graph as the tests of `graph.rs`. Do not copy
it. Move it into a module that only exists for tests.

- [ ] Make a new file `src/test_fixtures.rs`:

  ```rust
  //! Test data and helpers that more than one test module uses.

  use crate::graph::Graph;
  use crate::io::{EdgeRecord, GraphFile, NodeId, NodeRecord};

  pub const COORDS: [(NodeId, f64, f64); 6] = [
      (50, 0.0, 0.0),
      (2, 4.0, 0.0),
      (17, 0.0, 3.0),
      (8, 3.0, 4.0),
      (33, 20.0, 20.0),
      (7, 20.0, 20.0),
  ];

  pub const EDGES: [(NodeId, NodeId); 5] = [(50, 2), (50, 17), (17, 8), (2, 33), (33, 7)];

  pub fn assert_close(actual: f64, expected: f64) {
      assert!(
          (actual - expected).abs() < 1e-9,
          "expected {expected}, got {actual}"
      );
  }

  /// A `GraphFile` built from plain tuples.
  pub fn file(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> GraphFile {
      GraphFile {
          graph_id: "test".to_string(),
          nodes: nodes
              .iter()
              .map(|&(id, x, y)| NodeRecord { id, x, y })
              .collect(),
          edges: edges
              .iter()
              .map(|&(source, target)| EdgeRecord { source, target })
              .collect(),
      }
  }

  /// The six-node graph of the Python tests.
  pub fn fixture() -> Graph {
      Graph::from_file(&file(&COORDS, &EDGES)).unwrap()
  }
  ```

  This is the code from the test module of Slice 3, with `pub` on each item. `//!` is a doc
  comment for the whole file. `///` documents the item after it.

- [ ] In `src/lib.rs`, add two lines at the end:

  ```rust
  #[cfg(test)]
  mod test_fixtures;
  ```

  1. `#[cfg(test)]` on a `mod` line works as it does on `mod tests` (Slice 1, Step 7): the
     file is only compiled by `cargo test`. The normal build does not contain it.
  2. The module has no `pub`, so it is private to the crate. A private item can be used by
     its parent module and by everything inside that parent. Here the parent is the crate
     root, so every module of the crate can use it, and code outside the crate cannot. The
     `pub` on `fixture` and the others makes them visible to those modules.

- [ ] In the `mod tests` of `src/graph.rs`, delete `COORDS`, `EDGES`, `assert_close`, `file`
  and `fixture`. Keep `anchor_50`. Replace the line `use crate::io::{EdgeRecord, NodeRecord};`
  with:

  ```rust
      use crate::test_fixtures::{COORDS, assert_close, file, fixture};
  ```

  `EDGES` is not in the list, because the graph tests only use it through `fixture`.

- [ ] Run `cargo test graph`. Expected: `19 passed; 0 failed`, the same as before the move.

### 2. Tests for `subgraph`

`make_crop` needs a new kind of `Graph`: the nodes of a list of positions, and the edges
between them. This is `G.subgraph(nodes)` in networkx. It must be a method of `Graph`,
because only code in `graph.rs` can fill the private fields (Slice 3, Step 2).

- [ ] Add a stub inside `impl Graph`, after `radius_from`:

  ```rust
      /// The subgraph on the positions in `keep`, with every edge between two of them.
      ///
      /// Node `k` of the result is node `keep[k]` of `self`. Panics if `keep` has a
      /// repeated position.
      pub fn subgraph(&self, keep: &[usize]) -> Graph {
          todo!()
      }
  ```

  The order of `keep` is the order of the new graph. `connected_component` (Step 10) uses this
  to put its start node at position `0`. A repeated position would give the new graph two nodes with the same id.
  Only a bug in the caller can do that, so it panics (decision D3).

- [ ] Add three tests at the end of `mod tests` in `src/graph.rs`:

  ```rust
      #[test]
      fn subgraph_follows_the_order_of_keep() {
          let graph = fixture();
          let keep = [3, 2, 0]; // ids 8, 17, 50

          let sub = graph.subgraph(&keep);

          // node k of the subgraph is node keep[k] of the graph
          assert_eq!(sub.node_count(), 3);
          for (k, &i) in keep.iter().enumerate() {
              assert_eq!(sub.id(k), graph.id(i));
              assert_eq!(sub.coord(k), graph.coord(i));
              assert_eq!(sub.index_of(graph.id(i)), Some(k));
          }
      }

      #[test]
      fn subgraph_keeps_edges_between_kept_nodes() {
          let graph = fixture();

          let sub = graph.subgraph(&[3, 2, 0]); // ids 8, 17, 50

          // 8-17 and 17-50 stay; 50-2 goes, because node 2 is not kept
          assert_eq!(sub.edge_count(), 2);
          assert_eq!(sub.neighbors(0), &[1]);
          assert_eq!(sub.neighbors(1), &[0, 2]);
          assert_eq!(sub.neighbors(2), &[1]);
      }

      #[test]
      #[should_panic(expected = "repeated position")]
      fn subgraph_rejects_repeated_position() {
          let graph = fixture();

          // position 0 twice would give the subgraph two nodes with id 50
          graph.subgraph(&[0, 1, 0]);
      }
  ```

  The positions of the fixture are `50 → 0`, `2 → 1`, `17 → 2`, `8 → 3`, `33 → 4`, `7 → 5`.
  `keep` is in reverse order on purpose. In the new graph, node 17 is position 1, and its
  neighbours are 8 (new position 0) and 50 (new position 2). The test checks that the
  neighbour list is still sorted after the change of positions.

- [ ] Run `cargo test graph`. Expected: `19 passed; 3 failed`. All three fail with
  `not yet implemented`.

### 3. Write `subgraph`

**Try first.** Write the body. You need:

1. A `HashMap` from old position to new position, so that you can translate the neighbours.
2. The new `ids` and `coords`: for each position in `keep`, the old value.
3. The new adjacency: for each position in `keep`, its old neighbours that are also in the
   map, translated to new positions.
4. A new `index_of` from the new `ids`.

Two methods help. A `collect()` can build a `HashMap` from an iterator of `(key, value)`
pairs. `filter_map(f)` is a `map` and a `filter` in one step: it keeps `x` when `f` returns
`Some(x)`, and it drops the item when `f` returns `None`.

**Suggested code.** Start with this version. It has two bugs, and the tests find them.

```rust
    pub fn subgraph(&self, keep: &[usize]) -> Graph {
        let new_of: HashMap<usize, usize> = keep.iter().enumerate().map(|(k, &i)| (i, k)).collect();

        let ids: Vec<NodeId> = keep.iter().map(|&i| self.ids[i]).collect();
        let coords = keep.iter().map(|&i| self.coords[i]).collect();
        let adjacency = keep
            .iter()
            .map(|&i| {
                self.adjacency[i]
                    .iter()
                    .filter_map(|j| new_of.get(j).copied())
                    .collect()
            })
            .collect();
        let index_of = ids.iter().enumerate().map(|(k, &id)| (id, k)).collect();

        Graph {
            ids,
            coords,
            adjacency,
            index_of,
        }
    }
```

**Explanation.**

1. `keep.iter().enumerate()` gives `(new position, &old position)` pairs. The closure turns
   each pair around into `(old, new)`, and `collect()` builds the map. This is the Python
   `{i: k for k, i in enumerate(keep)}`. The `HashMap<usize, usize>` on the `let` line tells
   `collect` what to build (Slice 2, Step 10).
2. `coords` and `adjacency` have no type on their `let` line. The compiler finds their types
   from the fields of `Graph` at the end of the function. `ids` needs its type, because the
   line `ids.iter()` uses it before the compiler gets to the struct. Without the type, the
   error is `error[E0282]: type annotations needed`, with a note `type must be known at this
   point` under `ids.iter()`.
3. `.filter_map(|j| new_of.get(j).copied())` does the translation. `get` gives `Some(&new)`
   for a kept neighbour and `None` for a dropped one. `.copied()` turns the `Option<&usize>`
   into an `Option<usize>` (Slice 3, Step 6). So a kept neighbour is translated, and a
   neighbour outside `keep` is dropped. That is what "induced subgraph" means.
4. The inner `.collect()` builds one `Vec<usize>` for each kept node. The outer `.collect()`
   builds the `Vec<Vec<usize>>`.
5. `index_of` is built the same way as `new_of`, from the new `ids`.

- [ ] Run `cargo test subgraph`. Expected: `1 passed; 2 failed`.

  ```text
  ---- graph::tests::subgraph_rejects_repeated_position stdout ----
  note: test did not panic as expected
  ---- graph::tests::subgraph_keeps_edges_between_kept_nodes stdout ----
  assertion `left == right` failed
    left: [2, 0]
   right: [0, 2]
  ```

  The first failure: a repeated position does not panic. With `[0, 1, 0]`, the map gets the
  key `0` twice, and the second `insert` replaces the first. The map then has two keys, but
  `keep` has three positions. That difference is the check.

  The second failure: the neighbour list of node 17 is `[0, 3]` in the old graph. Position 0
  becomes 2, and position 3 becomes 0, so the new list is `[2, 0]`. Slice 3 promised sorted
  neighbour lists, so sort each new list.

- [ ] Add this line after the `let new_of` line:

  ```rust
        assert_eq!(new_of.len(), keep.len(), "keep has a repeated position");
  ```

  `assert_eq!` and `assert!` take an optional message after the values, in the same form as
  `println!`. The panic message then contains `keep has a repeated position`, and that is
  the text that `should_panic(expected = ...)` looks for. This check runs in release builds
  too. It costs one comparison.

- [ ] Replace the closure of the outer `map` with a block that sorts:

  ```rust
            .map(|&i| {
                let mut near: Vec<usize> = self.adjacency[i]
                    .iter()
                    .filter_map(|j| new_of.get(j).copied())
                    .collect();
                near.sort_unstable();
                near
            })
  ```

  A closure body can be a block with several lines, like a function body. The last line,
  `near`, is the value of the block (Slice 1, Step 9). `near` needs its type now, because
  `sort_unstable` uses it before the outer `collect`.

- [ ] Run `cargo test graph`. Expected: `22 passed; 0 failed`.

### 4. Add `kiddo`

- [ ] Run:

  ```powershell
  cargo add kiddo
  ```

  Expected: among other lines,

  ```text
        Adding kiddo v6.3.0 to dependencies
               Features:
               + fixed
               + multi-threaded
  ```

  `Cargo.toml` now has the line `kiddo = "6.3.0"`. Slice 1, Step 1 explains `cargo add` and
  features. The `multi-threaded` feature brings in `rayon`, the crate that Slice 10 uses for
  parallel search, so this is no extra cost for us.

- [ ] Run `cargo build`. The first build compiles kiddo and its dependencies, so it takes a
  while. Expected: it builds, with no new warnings.

### 5. The `SpatialIndex` struct

kiddo has two main tree types. A `MutableKdTree` lets you add and remove points. An
`ImmutableKdTree` is built once from a list of points and is faster to query. Our graph never
changes after it is built, so we use the immutable one. It gives each point an item number:
the point's place in the list. If the list is in graph order, the item number is the graph
position. Python needed the `node_ids` list for this.

**Try first.** Make `src/spatial_index.rs` and add `pub mod spatial_index;` to `src/lib.rs`. Write a
public struct `SpatialIndex` with two private fields: `graph: Graph` and
`tree: ImmutableKdTree<f64, 2>`. Then write `new(graph: Graph) -> SpatialIndex`, which makes a
`Vec<[f64; 2]>` of the coordinates in position order and builds the tree with
`ImmutableKdTree::new_from_slice(&points)`. Also write `graph(&self) -> &Graph`.

**Suggested code.**

```rust
use kiddo::ImmutableKdTree;

use crate::graph::Graph;

/// A graph together with a KD-tree over its node coordinates.
pub struct SpatialIndex {
    graph: Graph,
    tree: ImmutableKdTree<f64, 2>,
}

impl SpatialIndex {
    pub fn new(graph: Graph) -> SpatialIndex {
        let points: Vec<[f64; 2]> = (0..graph.node_count())
            .map(|i| {
                let p = graph.coord(i);
                [p.x, p.y]
            })
            .collect();
        let tree =
            ImmutableKdTree::new_from_slice(&points).expect("a graph has fewer than 2^32 nodes");

        SpatialIndex { graph, tree }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }
}
```

**Explanation.**

1. `ImmutableKdTree<f64, 2>` is a **generic type**. `Vec<usize>` and `HashMap<NodeId, usize>`
   are generic types too, and you have used them since Slice 1. The values in `<...>` are
   **type parameters**, like the `int` in the type hint `list[int]`. There is one
   difference from Python. The type checker of Python only reads the hint. The Rust compiler
   makes a separate copy of the tree code for `f64` and 2 dimensions, as if the code had been
   written by hand for those types. That is why generic code in Rust is as fast as code
   without parameters.
2. The `2` is a **const generic parameter**: a number that is part of the type. Because the
   dimension is fixed when the program compiles, a point is a `[f64; 2]` and not a `Vec`.
3. `[f64; 2]` is an **array**: a fixed number of values of one type, stored in place with no
   heap allocation. `[p.x, p.y]` makes one. You have seen arrays as the `COORDS` constant.
   The type of `COORDS` is `[(NodeId, f64, f64); 6]`.
4. `ImmutableKdTree` is a type alias (Slice 1, Step 3). In the kiddo source, it is:

   ```rust
   pub type ImmutableKdTree<AX, const K: usize> =
       KdTree<AX, u32, Eytzinger, VecOfArenas<AX, u32, K, 32>, K, 32>;
   ```

   The full type has seven parameters, for the memory layout of the tree. The alias chooses
   good values for six of them. Note the `u32`: it is the type of the item numbers. Step 7
   needs this.
5. `new_from_slice` returns a `Result`. Read the kiddo source and you find that it only fails
   when there are more points than a `u32` can number, which is about 4 billion. Our graphs
   have about 10^6 nodes, so a failure here is a bug, and `expect` is correct (decision D3,
   Slice 1, Step 11). The message says what we expected to be true.
6. `new(graph: Graph)` takes the graph **by value**. The caller's graph moves into the index
   (Slice 2, Step 3), and `SpatialIndex { graph, tree }` stores it. This replaces the
   `graph.copy()` of Python.
7. `graph(&self) -> &Graph` lends the graph to the caller, as `neighbors` lends a slice
   (Slice 3, Step 7). The caller can read the graph but cannot keep it longer than the index.
   A field and a method can have the same name (Slice 3, Step 6).

- [ ] Run `cargo build`. Expected: it builds, with the warning:

  ```text
  warning: field `tree` is never read
  ```

### 6. Stubs and the `query_radius` tests

- [ ] Add two stubs inside `impl SpatialIndex`, after `graph`. Also add
  `use crate::geometry::Point;` with the other `use` lines.

  ```rust
      pub fn query_radius(&self, point: Point, radius: f64) -> Vec<(usize, f64)> {
          todo!()
      }

      pub fn make_crop(&self, anchor: usize, radius: f64) -> Graph {
          todo!()
      }
  ```

  `query_radius` returns `(position, distance)` pairs, the same as `distance_from_node`.
  Python returns two separate lists, and the caller must keep them in step. One list of pairs
  cannot get out of step.

- [ ] Add the test module at the end of `src/spatial_index.rs`, with three helpers:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::io::NodeId;
      use crate::test_fixtures::{COORDS, assert_close, file, fixture};

      fn index() -> SpatialIndex {
          SpatialIndex::new(fixture())
      }

      /// The NodeIds of the positions in a query result.
      fn ids_of(index: &SpatialIndex, hits: &[(usize, f64)]) -> Vec<NodeId> {
          hits.iter().map(|&(i, _)| index.graph().id(i)).collect()
      }

      /// The ids within `radius` of `point`, found by checking every node.
      fn brute_force_within(point: Point, radius: f64) -> Vec<NodeId> {
          let mut ids: Vec<NodeId> = COORDS
              .iter()
              .filter(|&&(_, x, y)| Point::new(x, y).dist(point) <= radius)
              .map(|&(id, _, _)| id)
              .collect();
          ids.sort();
          ids
      }
  }
  ```

  1. `index()` is the `index` fixture of the Python file.
  2. `ids_of` turns positions into ids, so that a test can compare with the ids of the Python
     test.
  3. `brute_force_within` is the Python helper of the same name. It returns a sorted `Vec`
     and not a `set`. Two sorted lists are equal only if they have the same ids, and the same
     number of each, so the comparison also finds a repeated id. `filter` lends each item,
     and `iter` already gives references, so the pattern has two `&` (Slice 3, Step 10).

- [ ] Add the first four tests inside `mod tests`, after the helpers:

  ```rust
      #[test]
      fn every_node_finds_itself() {
          let index = index();
          let graph = index.graph();

          for i in 0..graph.node_count() {
              let hits = index.query_radius(graph.coord(i), 0.0);

              // tree item i is graph position i
              assert!(hits.iter().any(|&(j, _)| j == i));
          }
      }

      #[test]
      fn query_radius_distances_match_coordinates() {
          let index = index();
          let point = Point::new(1.0, 1.0);

          let hits = index.query_radius(point, 25.0);

          // every distance belongs to its own node
          assert!(!hits.is_empty());
          for (i, dist) in hits {
              assert_close(dist, index.graph().coord(i).dist(point));
          }
      }

      #[test]
      fn query_radius_sorts_by_distance() {
          let index = index();

          let hits = index.query_radius(Point::new(0.0, 0.0), 5.0);

          // nearest first, not in position order
          assert_eq!(ids_of(&index, &hits), vec![50, 17, 2, 8]);
          let dists: Vec<f64> = hits.iter().map(|&(_, d)| d).collect();
          assert_eq!(dists, vec![0.0, 3.0, 4.0, 5.0]);
      }

      #[test]
      fn query_radius_returns_exactly_nodes_within_radius() {
          let index = index();
          let point = Point::new(1.0, 2.0);

          let hits = index.query_radius(point, 6.0);

          // the same ids as a check of every node, and no repeats
          let mut ids = ids_of(&index, &hits);
          ids.sort();
          assert_eq!(ids, brute_force_within(point, 6.0));
      }
  ```

  1. `every_node_finds_itself` replaces the Python test that compares the KD-tree rows with
     `node_ids`. Our tree has no rows that a test can read, so the test checks the same promise
     through the public method: a query at the coordinates of node `i` finds position `i`.
     `any` is the Python `any`.
  2. `query_radius_distances_match_coordinates` checks `!hits.is_empty()` first. Without it,
     an empty result would pass the loop with no checks at all.
  3. `query_radius_sorts_by_distance` compares floats with `assert_eq!`, not
     `assert_close`. The square roots of 9, 16 and 25 are exact in floating point, so the
     exact compare is safe here.

- [ ] Add the other four tests:

  ```rust
      #[test]
      fn query_radius_boundary_is_inclusive() {
          let index = index();

          let hits = index.query_radius(Point::new(0.0, 0.0), 5.0);

          // node 8 is at exactly 5.0, and it is in the result
          let eight = index.graph().index_of(8).unwrap();
          let dist = hits.iter().find(|&&(i, _)| i == eight).unwrap().1;
          assert_close(dist, 5.0);
      }

      #[test]
      fn query_radius_at_node_coords_includes_that_node() {
          let index = index();

          let hits = index.query_radius(Point::new(0.0, 3.0), 1.0);

          // the node under the point comes first, at distance zero
          assert_eq!(ids_of(&index, &hits)[0], 17);
          assert_close(hits[0].1, 0.0);
      }

      #[test]
      fn query_radius_returns_coincident_nodes() {
          let index = index();

          let hits = index.query_radius(Point::new(20.0, 20.0), 0.5);

          // both nodes at (20, 20), in any order
          let mut ids = ids_of(&index, &hits);
          ids.sort();
          assert_eq!(ids, vec![7, 33]);
      }

      #[test]
      fn query_radius_with_no_hits_is_empty() {
          let index = index();

          let hits = index.query_radius(Point::new(1000.0, 1000.0), 1.0);

          assert!(hits.is_empty());
      }
  ```

  1. `query_radius_returns_coincident_nodes` sorts the ids before the compare. The two nodes
     are at the same distance, and their order is not promised (see "The design").
  2. In the Python test, an empty result is "two empty lists". In Rust it is one empty `Vec`.

- [ ] Run `cargo test spatial`. Expected: `0 passed; 8 failed`, all with `not yet implemented`.
  You also see warnings about unused variables in the stubs.

### 7. Write `query_radius`

**Try first.** A kiddo query is a chain of method calls. Each call adds one setting, and the
last call runs the query:

```rust
self.tree
    .query(&[x, y])                              // the query point
    .within::<SquaredEuclidean<f64>>(max_dist)   // all points within max_dist, nearest first
    .execute()                                   // run it: a Vec of results
```

Each result has the fields `item` (a `u32`, the position) and `distance`. Write
`query_radius` with this chain, and turn the results into `(position, distance)` pairs. Two
details:

- `SquaredEuclidean` measures the **square** of the distance. Give it `radius * radius`, and
  take the square root of each result distance.
- `item` is a `u32`, and a position is a `usize`. `x as usize` converts a number to another
  number type.

You need `use kiddo::{ImmutableKdTree, SquaredEuclidean};` at the top.

**Suggested code.**

```rust
    pub fn query_radius(&self, point: Point, radius: f64) -> Vec<(usize, f64)> {
        let found = self
            .tree
            .query(&[point.x, point.y])
            .within::<SquaredEuclidean<f64>>(radius * radius)
            .execute();

        found
            .iter()
            .map(|hit| (hit.item as usize, hit.distance.sqrt()))
            .collect()
    }
```

**Explanation.**

1. The chain is a **builder**. `query` returns an object that holds the settings. Each later
   method returns an object with one more setting, and `execute` runs the query. This is
   common in Rust when a function has many options, because Rust has no keyword arguments
   (Slice 2, Step 13). In Python, the same query would be one call with keywords:
   `tree.query_ball_point(point, r=radius)`.
2. `within::<SquaredEuclidean<f64>>` chooses the distance measure with a turbofish (Slice 3,
   Step 8). The measure is a type, and not a value, so the compiler makes code for exactly
   this measure.
3. Why squared? The square root is the slow part of a distance. A KD-tree compares many
   distances, and `d² <= r²` gives the same answer as `d <= r` for distances that are not
   negative. So kiddo compares squares, and we take a square root only for the results.
4. kiddo includes a point at exactly `max_dist`. This is the `<=` of the Python behaviour
   that the port must keep.
5. `hit.item as usize` is a **cast**. `as` converts between number types. Rust never does
   this for you, and the experiment below shows the error. Be careful with `as`: it never
   fails, and when the value does not fit it changes the value silently (`300_u32 as u8` is
   `44`). Here every `u32` fits in a 64-bit `usize`, so the cast is safe.
6. `found.iter()` lends each result, so `hit` is a reference. Fields of a reference can be
   read directly (Slice 3, Step 6).
7. `within` returns the results nearest first. Python sorts them itself, because scipy
   returns them in index order. kiddo also has `.unsorted()`, which skips the sort.

- [ ] Optional experiment. Remove `as usize` and run `cargo build`. Expected:

  ```text
  error[E0277]: a value of type `Vec<(usize, f64)>` cannot be built from an iterator over elements of type `(u32, f64)`
  ```

  Put `as usize` back.

- [ ] Run `cargo test spatial`. Expected: `8 passed; 0 failed`. You still see the warnings for
  the unused variables of `make_crop`.

### 8. The `make_crop` tests

These tests check what `make_crop` itself does: which nodes are in the crop, and which node
is first. The edges and the coordinates of a crop come from `subgraph`, and Step 2 tests
them.

- [ ] Add a helper at the end of `mod tests` in `src/spatial_index.rs`:

  ```rust
      /// The ids of a graph, sorted.
      fn sorted_ids(graph: &Graph) -> Vec<NodeId> {
          let mut ids: Vec<NodeId> = (0..graph.node_count()).map(|i| graph.id(i)).collect();
          ids.sort();
          ids
      }
  ```

  Only the position of the anchor is promised, so the tests compare sorted ids.

- [ ] Add the five tests:

  ```rust
      #[test]
      fn crop_has_the_nodes_within_the_radius() {
          let index = index();

          let crop = index.make_crop(0, 5.0);

          // the same ids as a check of every node, anchor included
          assert_eq!(
              sorted_ids(&crop),
              brute_force_within(Point::new(0.0, 0.0), 5.0)
          );
      }

      #[test]
      fn crop_starts_with_the_anchor() {
          let index = index();
          let graph = index.graph();

          for anchor in 0..graph.node_count() {
              let crop = index.make_crop(anchor, 5.0);

              // the anchor is position 0 of every crop
              assert_eq!(crop.id(0), graph.id(anchor));
          }
      }

      #[test]
      fn crop_with_radius_zero_is_the_anchor() {
          let index = index();

          let crop = index.make_crop(0, 0.0);

          // only node 50 is at distance zero from itself
          assert_eq!(sorted_ids(&crop), vec![50]);
      }

      #[test]
      fn crop_with_radius_zero_keeps_coincident_nodes() {
          let index = index();
          let anchor = index.graph().index_of(33).unwrap();

          let crop = index.make_crop(anchor, 0.0);

          // node 7 is at the same point as node 33, and they share an edge
          assert_eq!(sorted_ids(&crop), vec![7, 33]);
      }

      #[test]
      fn crop_drops_nodes_connected_only_outside_the_radius() {
          // 1 and 3 are inside the radius, but the path 1-2-3 goes through 2, which is not
          let nodes = [(1, 0.0, 0.0), (2, 10.0, 0.0), (3, 1.0, 0.0)];
          let graph = Graph::from_file(&file(&nodes, &[(1, 2), (2, 3)])).unwrap();
          let index = SpatialIndex::new(graph);

          let crop = index.make_crop(0, 5.0);

          assert_eq!(sorted_ids(&crop), vec![1]);
      }
  ```

  1. The anchor of most tests is position `0`, which is node 50.
  2. `crop_starts_with_the_anchor` is new. It makes a crop around every node of the fixture.
     The hard case is node 7: node 33 is at the same point, so the query can return 33 first.
  3. `crop_drops_nodes_connected_only_outside_the_radius` is the `A-B-C` example of "The
     design". In the Python test, the extra node has no edges at all. This graph is
     stricter: it also finds a component search on the full graph instead of the circle.

- [ ] Run `cargo test spatial`. Expected: `8 passed; 5 failed`. The new tests fail with
  `not yet implemented`.

### 9. A first `make_crop`

Do only step 1 of "The design": crop to the radius, with no component search. The tests then
show what step 2 must add.

- [ ] Replace the stub of `make_crop`:

  ```rust
      pub fn make_crop(&self, anchor: usize, radius: f64) -> Graph {
          let hits = self.query_radius(self.graph.coord(anchor), radius);
          let in_circle: Vec<usize> = hits.iter().map(|&(i, _)| i).collect();
          self.graph.subgraph(&in_circle)
      }
  ```

  This is the first half of the Python function: `query_radius` around the anchor, then
  `self.graph.subgraph(neighbourhood)`.

- [ ] Run `cargo test crop`. Expected: `3 passed; 2 failed`.

  ```text
  ---- spatial_index::tests::crop_drops_nodes_connected_only_outside_the_radius stdout ----
    left: [1, 3]
   right: [1]
  ---- spatial_index::tests::crop_starts_with_the_anchor stdout ----
    left: 33
   right: 7
  ```

  The first failure is node 3. It is inside the radius, but its only path to the anchor goes
  outside. The second failure is the coincident pair. The anchor is node 7, but the query
  returned node 33 first, at the same distance of zero. So the query order cannot put the
  anchor first. (Your tie order can be different. Then only the first test fails.)

### 10. `connected_component`

The Python code calls `nx.node_connected_component`. We write it ourselves (decision D5), as a
method of `Graph` that returns the component as a new `Graph`.

The search keeps a stack of nodes to visit, and it remembers which nodes it has seen. It takes
a node off the stack and puts each neighbour that it has not seen yet on the stack. When the
stack is empty, it has seen every node that a path from the start reaches. In Python:

```python
seen = [False] * graph.number_of_nodes()
seen[start] = True
keep, stack = [start], [start]
while stack:
    i = stack.pop()
    for j in graph.neighbors(i):
        if not seen[j]:
            seen[j] = True
            keep.append(j)
            stack.append(j)
```

The order of the visit does not matter, because only the start node must be first. A stack is
a plain `Vec`, with `push` and `pop`. A queue gives a breadth-first search. That needs a
`VecDeque` and finds the same nodes.

- [ ] Add a stub inside `impl Graph` in `src/graph.rs`, after `subgraph`:

  ```rust
      /// The component of `start`: every node that a path from `start` reaches.
      ///
      /// `start` is position `0` of the result.
      pub fn connected_component(&self, start: usize) -> Graph {
          todo!()
      }
  ```

- [ ] Add three tests at the end of `mod tests` in `src/graph.rs`:

  ```rust
      #[test]
      fn component_starts_with_start() {
          let graph = fixture();

          let component = graph.connected_component(2); // id 17

          // the fixture is one component, so every node is in it
          assert_eq!(component.id(0), 17);
          assert_eq!(component.node_count(), 6);
          assert_eq!(component.edge_count(), 5);
      }

      #[test]
      fn component_keeps_only_connected_nodes() {
          // 1-2-3 is a path; 100 has no edge
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0), (100, 3.0, 0.0)];
          let graph = Graph::from_file(&file(&nodes, &[(1, 2), (2, 3)])).unwrap();

          let component = graph.connected_component(0);

          let mut ids: Vec<NodeId> = (0..component.node_count())
              .map(|i| component.id(i))
              .collect();
          ids.sort();
          assert_eq!(ids, vec![1, 2, 3]);
          assert_eq!(component.edge_count(), 2);
      }

      #[test]
      fn component_of_isolated_node_is_that_node() {
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (100, 3.0, 0.0)];
          let graph = Graph::from_file(&file(&nodes, &[(1, 2)])).unwrap();

          let component = graph.connected_component(2); // id 100

          assert_eq!(component.node_count(), 1);
          assert_eq!(component.id(0), 100);
          assert_eq!(component.edge_count(), 0);
      }
  ```

  The fixture is one component, because the edge 2-33 joins the two groups.
  `component_starts_with_start` starts at node 17, which is not position `0`, so the test
  can see that the start goes first.

- [ ] Run `cargo test graph`. Expected: `22 passed; 3 failed`, all with `not yet implemented`.

**Try first.** Write the body. A `Vec<bool>` holds `seen`. `Vec::pop()` returns an `Option`:
`Some(last)`, or `None` when the `Vec` is empty. The last line calls `subgraph`.

**Suggested code.**

```rust
    pub fn connected_component(&self, start: usize) -> Graph {
        let mut seen = vec![false; self.node_count()];
        seen[start] = true;

        let mut keep = vec![start];
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            for &j in self.neighbors(i) {
                if !seen[j] {
                    seen[j] = true;
                    keep.push(j);
                    stack.push(j);
                }
            }
        }

        self.subgraph(&keep)
    }
```

**Explanation.**

1. `vec![false; self.node_count()]` makes a `Vec` with `node_count()` copies of `false`. It is
   the Python `[False] * n`. The positions are `0..n`, so a position is also an index into
   `seen`. This is faster than a `HashSet`, which must hash each position.
2. `while let Some(i) = stack.pop()` is a loop with a pattern. Each time round, it calls `pop`
   and tries to match the result with `Some(i)`. If it matches, the value goes into `i` and
   the body runs. When the stack is empty, `pop` returns `None`, the pattern does not match,
   and the loop stops. It replaces the Python `while stack: i = stack.pop()`, and you cannot
   forget the check for an empty stack.
3. A node is marked as seen when it goes onto the stack, not when it comes off. So no node goes
   onto the stack twice, and `keep` never has a repeated position. `subgraph` panics on a
   repeated position.
4. `for &j in self.neighbors(i)` loops over the slice that `neighbors` lends. `&j` copies each
   `usize` out of the reference.
5. `keep` starts with `start`, so `subgraph` puts it at position `0`.

- [ ] Run `cargo test graph`. Expected: `25 passed; 0 failed`.

### 11. Finish `make_crop`

**Try first.** Add step 2 of "The design" to `make_crop`. Keep the circle in a variable, and
return its component.

The obvious version has a bug. Write it first, and let the tests find the bug:

- [ ] Change the last line of `make_crop` to:

  ```rust
          let circle = self.graph.subgraph(&in_circle);
          circle.connected_component(anchor)
  ```

- [ ] Run `cargo test crop`. Expected: `3 passed; 2 failed`.

  ```text
  ---- spatial_index::tests::crop_starts_with_the_anchor stdout ----
    left: 50
   right: 2
  ---- spatial_index::tests::crop_with_radius_zero_keeps_coincident_nodes stdout ----
  index out of bounds: the len is 2 but the index is 4
  ```

  `anchor` is a position in `self.graph`, but `connected_component` takes a position in
  `circle`. The two graphs have different positions. In the first failure, the anchor is
  node 2, at position 1 of the graph. In the circle, position 1 is node 50, so the search
  starts at 50. In the second failure, the anchor is node 33, at position 4. The circle only
  has 2 nodes, so `seen[4]` panics.

  The compiler cannot find this bug: both positions are `usize`.

- [ ] Translate the anchor to its position in the circle:

  ```rust
      pub fn make_crop(&self, anchor: usize, radius: f64) -> Graph {
          let hits = self.query_radius(self.graph.coord(anchor), radius);
          let in_circle: Vec<usize> = hits.iter().map(|&(i, _)| i).collect();
          let circle = self.graph.subgraph(&in_circle);

          // the anchor has a new position in the circle
          let start = circle.index_of(self.graph.id(anchor)).unwrap();
          circle.connected_component(start)
      }
  ```

  1. The NodeId is the same in both graphs, so it carries the anchor from one graph to the
     other. `self.graph.id(anchor)` gives the NodeId, and `circle.index_of` gives its
     position in the circle. This is one reason why `subgraph` keeps the NodeIds.
  2. Do not use `0` as the start. Step 9 showed that a coincident node can come first.
  3. The anchor is at distance zero from itself, so it is always in the circle. A `None` here
     is a bug, so `unwrap` is correct (decision D3).

- [ ] Run `cargo test`. Expected: `51 passed; 0 failed`. That is 13 tests from Slices 1 and 2,
  25 graph tests and 13 spatial tests.

### 12. Look at it

- [ ] Add a temporary test at the end of `mod tests` in `src/spatial_index.rs`:

  ```rust
      #[test]
      fn look() {
          let index = index();
          let crop = index.make_crop(0, 5.0);
          for i in 0..crop.node_count() {
              let near: Vec<NodeId> = crop.neighbors(i).iter().map(|&j| crop.id(j)).collect();
              println!("{i}: id {} at {:?}, neighbours {near:?}", crop.id(i), crop.coord(i));
          }
      }
  ```

- [ ] Run `cargo test look -- --nocapture`. Expected:

  ```text
  0: id 50 at Point { x: 0.0, y: 0.0 }, neighbours [17, 2]
  1: id 17 at Point { x: 0.0, y: 3.0 }, neighbours [50, 8]
  2: id 2 at Point { x: 4.0, y: 0.0 }, neighbours [50]
  3: id 8 at Point { x: 3.0, y: 4.0 }, neighbours [17]
  ```

  Compare it with the table at the top of `test_spatial.py`. Node 2 lost its edge to 33,
  because 33 is outside the radius. Only the first line is fixed: the anchor is position `0`.
  The order of the other lines comes from the query and the search.

  `cargo test` normally hides what a passing test prints, like `pytest` without `-s`.
  `--nocapture` shows it. The `--` before it means "the rest is for the test program, not for
  cargo".

- [ ] Delete the `look` test.

### 13. Use it from `main`

The program now takes two files: a query and a graph. It computes the crop radius from the
query, as `run_gsearch` does (`Q_radius + tol`), crops the graph around its first node, and
then crops around every node, as the search loop of `run_gsearch` will.

- [ ] Replace all of `src/main.rs`:

  ```rust
  use std::error::Error;
  use std::time::Instant;

  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;
  use gsearch::spatial_index::SpatialIndex;

  /// The default tolerance of the Python CLI. Slice 8 makes it an argument.
  const TOL: f64 = 10.0;

  fn main() -> Result<(), Box<dyn Error>> {
      let usage = "usage: gsearch <query.json> <graph.json>";
      let mut args = std::env::args().skip(1);
      let query_path = args.next().ok_or(usage)?;
      let graph_path = args.next().ok_or(usage)?;

      let query_file = read_graph_file(&query_path)?;
      let query = Graph::from_file(&query_file)?;
      let radius = query.radius_from(0) + TOL;
      println!(
          "{}: {} nodes, crop radius {:.4}",
          query_file.graph_id,
          query.node_count(),
          radius,
      );

      let start = Instant::now();
      let graph_file = read_graph_file(&graph_path)?;
      let graph = Graph::from_file(&graph_file)?;
      let index = SpatialIndex::new(graph);
      eprintln!("load and index:  {:.3} s", start.elapsed().as_secs_f64());

      let graph = index.graph();
      println!(
          "{}: {} nodes, {} edges",
          graph_file.graph_id,
          graph.node_count(),
          graph.edge_count(),
      );

      let crop = index.make_crop(0, radius);
      println!(
          "crop around node {}: {} nodes, {} edges",
          graph.id(0),
          crop.node_count(),
          crop.edge_count(),
      );

      let start = Instant::now();
      let mut total = 0;
      for anchor in 0..graph.node_count() {
          total += index.make_crop(anchor, radius).node_count();
      }
      eprintln!("crop every node: {:.3} s", start.elapsed().as_secs_f64());
      println!(
          "mean crop size: {:.2} nodes",
          total as f64 / graph.node_count() as f64
      );
      Ok(())
  }
  ```

  1. `std::env::args()` is an iterator over the arguments, and the first one is the program
     name. `.skip(1)` drops it. `args.next()` takes the next item, like `next(it)` in Python,
     and returns an `Option`. The iterator changes each time it gives an item, so `args`
     must be `mut`. Slice 1 used `.nth(1)`, which only works for one argument.
  2. `.ok_or(usage)?` turns a missing argument into an error with the usage text (Slice 1,
     Step 14). `usage` is used twice. A `&str` is `Copy`, so that is allowed.
  3. `let graph = index.graph();` uses the name `graph` again. The new `graph` is a `&Graph`
     borrowed from the index. The old one was moved into the index on the line before. Rust
     allows a new `let` with an old name. This is **shadowing**, and it is common when a value
     changes form. Slice 3, Step 14 did the same with `start`.
  4. `total` has no type written. The compiler infers `usize` from `+= node_count()`.
     `total as f64` and `graph.node_count() as f64` cast both numbers to `f64` before the
     division. Rust does not divide a `usize` by an `f64`, and a division of two `usize`
     values would drop the fraction.
  5. The loop throws each crop away after it reads its size. The memory of each crop is freed
     at the end of its loop step, when the crop goes out of scope.

- [ ] Run:

  ```powershell
  cargo run -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json
  ```

  Expected (your times will be a little different):

  ```text
  gr00049_q: 10 nodes, crop radius 68.0499
  load and index:  0.005 s
  gr00049: 1128 nodes, 1205 edges
  crop around node 0: 5 nodes, 4 edges
  crop every node: 0.040 s
  mean crop size: 12.86 nodes
  ```

  The first node of `gr00049.json` has the id `0`. That is a coincidence of this file, not a
  position.

- [ ] Run it with only one file: `cargo run -- data\roads\queries\gr00049_q.json`. Expected:

  ```text
  Error: "usage: gsearch <query.json> <graph.json>"
  ```

### 14. Experiment: the index owns the graph

This is the Rust form of `test_init_copies_graph`.

- [ ] In `src/main.rs`, add a line after `let index = SpatialIndex::new(graph);`:

  ```rust
      println!("{}", graph.node_count());
  ```

- [ ] Run `cargo build`. Expected: the build fails.

  ```text
  error[E0382]: borrow of moved value: `graph`
     |
  29 |     let graph = Graph::from_file(&graph_file)?;
     |         ----- move occurs because `graph` has type `Graph`, which does not implement the `Copy` trait
  30 |     let index = SpatialIndex::new(graph);
     |                                   ----- value moved here
  31 |     println!("{}", graph.node_count());
     |                    ^^^^^ value borrowed here after move
  ```

  This is the error of Slice 2, Step 3, now on a large type. The Python test checks that a
  change to the original graph does not reach the index. In Rust, you cannot even read the
  original after the move, so you cannot change it. If you need the graph in two places, you
  write `graph.clone()`, and the copy is visible in the code. (`Graph` does not derive `Clone`
  yet, so that also needs a `#[derive(Clone)]`.)

- [ ] Remove the line.

### 15. Time it, and compare with Python

- [ ] Build in release mode and run all three road graphs:

  ```powershell
  cargo build --release
  .\target\release\gsearch.exe data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json
  .\target\release\gsearch.exe data\roads\queries\gr00048_q.json data\roads\graphs\gr00048.json
  .\target\release\gsearch.exe data\roads\queries\gr00047_q.json data\roads\graphs\gr00047.json
  ```

  Expected:

  ```text
  gr00049_q: 10 nodes, crop radius 68.0499
  load and index:  0.001 s
  gr00049: 1128 nodes, 1205 edges
  crop around node 0: 5 nodes, 4 edges
  crop every node: 0.007 s
  mean crop size: 12.86 nodes
  gr00048_q: 19 nodes, crop radius 279.6843
  load and index:  0.001 s
  gr00048: 1607 nodes, 1660 edges
  crop around node 0: 12 nodes, 11 edges
  crop every node: 0.032 s
  mean crop size: 101.04 nodes
  gr00047_q: 11 nodes, crop radius 242.3581
  load and index:  0.043 s
  gr00047: 100110 nodes, 106210 edges
  crop around node 0: 43 nodes, 42 edges
  crop every node: 1.986 s
  mean crop size: 94.56 nodes
  ```

  For comparison, the debug build takes about 23 seconds to crop every node of `gr00047`.
  Always time the release build (Slice 0, Step 7).

- [ ] Run the same check in Python. This script uses the Python functions themselves, so it
  also checks that the two ports make the same crops. From the `gsearch` folder:

  ```powershell
  cd ..\gsearch
  $check = @'
  import sys, time
  from gsearch.core import _get_radius
  from gsearch.io import load_graph
  from gsearch.spatial_graph_index import SpatialGraphIndex

  q, g = load_graph(sys.argv[1]), load_graph(sys.argv[2])
  radius = _get_radius(q, next(iter(q))) + 10.0
  index = SpatialGraphIndex(g)
  crop = index.make_crop(next(iter(g)), radius)
  print(f"crop around node {next(iter(g))}: {crop.number_of_nodes()} nodes, {crop.number_of_edges()} edges")
  start = time.perf_counter()
  total = sum(index.make_crop(n, radius).number_of_nodes() for n in g)
  print(f"crop every node: {time.perf_counter() - start:.3f} s")
  print(f"mean crop size: {total / g.number_of_nodes():.2f} nodes")
  '@
  $check | uv run python - ..\gsearch_rs\data\roads\queries\gr00049_q.json ..\gsearch_rs\data\roads\graphs\gr00049.json
  cd ..\gsearch_rs
  ```

  Expected:

  ```text
  crop around node 0: 5 nodes, 4 edges
  crop every node: 0.278 s
  mean crop size: 12.86 nodes
  ```

  The crop around node 0 and the mean crop size are the same as in Rust. The mean is taken
  over all 1128 crops, so a wrong crop anywhere would very likely change it.

  `@'...'@` is a PowerShell here-string, and `python -` reads the script from the pipe.

- [ ] Optional: run the Python script on `gr00047` too (change both paths). It takes about
  2.5 minutes. Expected: `crop every node: 148.848 s` (about) and `mean crop size: 94.56
  nodes`. Rust does the same work in about 2 seconds, about 75 times faster, on one thread.

Most of the Rust time is memory allocation: every crop builds two `Graph`s (the circle and its
component), each with its own `Vec`s and `HashMap`s, and a `seen` list the size of the circle. Slice 9 measures this. Slice 10 runs the crops on
all cores.

### 16. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
- [ ] Run `cargo test`. Expected: `51 passed; 0 failed`.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 4: spatial index with kiddo, subgraph, make_crop"
  ```

---

## What comes next

Slice 5 ports `align_nodes.py`: annulus masks, the split by handedness, and the injective
assignments of `get_candidate_matches`. It works on `Q` and on a crop. Both have their anchor
at position `0`, and both are small (the mean crop above has fewer than 100 nodes), so the
masks are plain loops over two small graphs (decision D4).

Two things in this slice are deliberately simple, and Slice 9 comes back to them:

- `make_crop` sorts the query results, but the crop does not need the sort. It only needs the
  set of positions. `.unsorted()` skips it.
- `make_crop` builds the circle as a full `Graph` and then throws it away. A search on the
  big graph that only steps onto nodes inside the radius gives the same crop with one
  `Graph`.
- That search could also test the distance of each neighbour directly, with no tree
  at all. That visits only the nodes that are connected to the anchor. Slice 9 measures if it
  is faster than the KD-tree query.
