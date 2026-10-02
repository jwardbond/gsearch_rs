# Slice 3: The graph type

**Goal.** Replace the `nx.Graph` that `io.load_graph` builds with a `Graph` struct of your own
(decision D5), and port the two functions of `core.py` that work on it: `_distance_from_node`
and `_get_radius`. The graph is built from the `GraphFile` of Slice 1. Inside the graph, every
node is a position `0..n`, and the ids from the file are only used at the edges (decision D7).

**Outcome.** `cargo test graph` reports `19 passed`, and this works:

```text
> cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
gd00000_q: 3 nodes, 3 edges
anchor radius: 5.7793
```

In a release build, reading the 238 MB `go00001.json` and building its graph takes about one
second. The Python `load_graph` takes about 19 seconds on the same file.

**New ideas.** Private fields; `HashMap`; `Vec::with_capacity` and `vec!`; enums that hold data;
the `Display` and `Error` traits; `&self` methods; slices (`&[T]`); `Option::copied`; closures
that capture variables; `?` with your own error type; `sort`, `dedup` and `&mut` loops; tuples;
ranges and `filter`; `sort_by` and `total_cmp`; `fold`; `#[should_panic]`; `Instant`.

**Starting point.** Slice 2 is done. `Point`, `Point::dist` and the other geometry functions
work, and `cargo test` passes.

---

## The design

`nx.Graph` is a dict of dicts. The node ids are the keys, and the coordinates are small dicts
(`G.nodes[n]["x"]`). That is flexible, but every node and every edge is a separate Python
object, and that is why a graph with 10^6 nodes is slow and large.

The `Graph` of this slice stores four plain lists. Node number `i` is the position `i` in each
list:

```text
position     0        1        2        3        4          5
ids          50       2        17       8        33         7
coords       (0, 0)   (4, 0)   (0, 3)   (3, 4)   (20, 20)   (20, 20)
adjacency    [1, 2]   [0, 4]   [0, 3]   [2]      [1, 5]     [4]
```

The table is the test graph of `tests/test_core.py`. A fourth structure, `index_of`, is a
dictionary from file id to position: `{50: 0, 2: 1, 17: 2, ...}`. The code uses `index_of` once
at the start, to turn an id from the file into a position. After that, everything uses
positions. The Python code does the same job with `list.index(...)` and
`{n: i for i, n in enumerate(...)}` in many places.

There is one more benefit. A file id has the type `NodeId` (`u64`) and a position has the type
`usize`. These are two different types, so the compiler rejects a call that mixes them. The
Python test file says that its ids are "deliberately non-contiguous and out of order so that a
mix-up between an insertion index and a node id cannot pass unnoticed". In Rust, most of these
mix-ups cannot compile.

### What the builder does with bad input

`nx.Graph` accepts anything. Our builder is stricter, and this is a choice that you can change.
None of these cases occurs in the data: I checked the 50 query files, `gg00041.json` and
`go00001.json`.

| Case in the file | `nx.Graph` | `Graph::from_file` |
|---|---|---|
| Two nodes with the same id | The second node updates the first | Error: `DuplicateNode` |
| An edge to an id that has no node | Creates a node with no `x` or `y`, and a `KeyError` follows later | Error: `UnknownNode` |
| The same edge twice, or in both directions | One edge | One edge |
| An edge from a node to itself | Keeps the self-loop | Drops it |

---

## Steps

### 1. Add the module

- [ ] In `src/lib.rs`, add `pub mod graph;`. The file now has three lines:

  ```rust
  pub mod geometry;
  pub mod graph;
  pub mod io;
  ```

  (Your order may differ. The order does not matter.)

- [ ] Make an empty file `src/graph.rs`.

### 2. The struct

**Try first.** In `src/graph.rs`, write a public struct `Graph` with four fields:

- `ids`: a list of `NodeId`.
- `coords`: a list of `Point`.
- `adjacency`: a list that has, for each node, a list of positions (`usize`).
- `index_of`: a `HashMap` from `NodeId` to `usize`.

Do not write `pub` on the fields. The reason is in the explanation.

**Suggested code.**

```rust
use std::collections::HashMap;

use crate::geometry::Point;
use crate::io::NodeId;

/// An undirected graph with 2D node coordinates.
///
/// Every node is a position `0..node_count()`. File ids only appear in `id` and `index_of`.
pub struct Graph {
    ids: Vec<NodeId>,
    coords: Vec<Point>,
    adjacency: Vec<Vec<usize>>,
    index_of: HashMap<NodeId, usize>,
}
```

**Explanation.**

1. `use crate::geometry::Point;` imports from your own crate. `crate` means "the root of this
   crate", so the line is like `from gsearch.geometry import Point`. Slice 1, Step 2 explains
   `use`.
2. There are four parallel `Vec`s, and not one `Vec` of node structs. A loop that only needs
   coordinates (and most of the matching code is such a loop) then reads one block of memory
   with nothing else mixed in. This is the "own compact struct" of decision D5.
3. `Vec<Vec<usize>>` is a list of lists, one list of neighbours for each node. `usize` is the
   unsigned integer type that is as wide as a pointer (64 bits on your computer). It is the
   type that Rust uses for positions: `v[i]` only accepts a `usize`.
4. `HashMap<NodeId, usize>` is a `dict[int, int]`. Rust needs the key and value types, and
   they are fixed for the whole map.
5. The fields have no `pub`, so they are **private**: only code in this module can touch them
   (Slice 1, Step 2 says that everything is private by default). This is how we protect the
   rules of the graph: the four lists have the same length, and `a` is in the list of `b` if
   and only if `b` is in the list of `a`. Other code can read the graph through methods, so
   it cannot break these rules. Python uses a leading `_` as a polite request. Rust makes the
   compiler enforce it.

- [ ] Run `cargo build`. Expected: it builds, with one warning:

  ```text
  warning: fields `ids`, `coords`, `adjacency`, and `index_of` are never read
  ```

  The compiler found that nothing uses the private fields. The warning goes away when we use
  them.

### 3. The error type

Building a graph from a file can fail because of the data in the file (a repeated id, an edge
to a missing node). This is not a bug in the program, so the function returns a `Result`
(decision D3). Slice 1 used `Box<dyn Error>`, which holds any error. Here we make an error
type of our own, so that a test can check which error came back.

**Try first.** Write a public enum `GraphError` with two variants, `DuplicateNode` and
`UnknownNode`. Each variant holds one `NodeId`. Derive `Debug` and `PartialEq`.

**Suggested code.** Add `use std::error::Error;` and `use std::fmt;` at the top of the file,
with the other `use` lines. Then add this after the struct:

```rust
/// Why a `GraphFile` cannot become a `Graph`.
#[derive(Debug, PartialEq)]
pub enum GraphError {
    DuplicateNode(NodeId),
    UnknownNode(NodeId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GraphError::DuplicateNode(id) => write!(f, "node id {id} appears twice"),
            GraphError::UnknownNode(id) => {
                write!(f, "an edge uses node id {id}, which has no node")
            }
        }
    }
}

impl Error for GraphError {}
```

**Explanation.**

1. In Slice 2, Step 13, the variants of `Side` held no data. Here each variant **holds a value**.
   `GraphError::DuplicateNode(1)` means "the id 1 was repeated". It is like a Python exception
   class that has an attribute, but the cases of the error are all part of one type. Code that
   gets a `GraphError` uses `match` to see which case it is and to take out the id.
2. `Debug` is for the programmer (`{:?}`), and `Display` is for the user (`{}`). They are the
   `__repr__` and `__str__` of Python. `#[derive(Debug)]` writes the first one. We write
   `Display` by hand, because only we know the correct message.
3. `fmt(&self, f: &mut fmt::Formatter) -> fmt::Result` is the one method that `Display`
   needs. `&self` borrows the error for reading. `&mut` is a **mutable borrow**: the function
   is allowed to change `*f`, and `write!` writes the text into it. `write!(f, "...")` works
   like `println!`, but the text goes into `f`. The `{id}` in the string prints the variable
   `id` that the `match` arm created.
4. `impl Error for GraphError {}` has an empty body. The `Error` trait has default code for all
   its methods, so this line only says "this type is an error". Rust makes you say it, and
   the `Error` trait requires that the type has both `Debug` and `Display`.
5. This line is also what lets `?` convert a `GraphError` into a `Box<dyn Error>` (Slice 1,
   Step 13). You use it in Step 13.

The `thiserror` crate writes the `Display` code from an attribute. We write it by hand once,
so that you see what the attribute would hide. Slice 8 may switch to a crate.

- [ ] Run `cargo build`. Expected: the same warning as before, and no new one.
- [ ] Optional. Delete the `impl fmt::Display` block and run `cargo build`. Expected:
  `error[E0277]: GraphError doesn't implement std::fmt::Display`, and a note that says
  `pub trait Error: Debug + Display`. Put the block back.

### 4. Stubs and the first tests

**Try first.** Add an `impl Graph` block that has five methods with the body `todo!()`:

| Method | Takes | Returns |
|---|---|---|
| `from_file` | `file: &GraphFile` | `Result<Graph, GraphError>` |
| `node_count` | `&self` | `usize` |
| `id` | `&self`, `i: usize` | `NodeId` |
| `coord` | `&self`, `i: usize` | `Point` |
| `index_of` | `&self`, `id: NodeId` | `Option<usize>` |

**Suggested code.** Change the last `use` line to `use crate::io::{GraphFile, NodeId};` and add:

```rust
impl Graph {
    pub fn from_file(file: &GraphFile) -> Result<Graph, GraphError> {
        todo!()
    }

    pub fn node_count(&self) -> usize {
        todo!()
    }

    pub fn id(&self, i: usize) -> NodeId {
        todo!()
    }

    pub fn coord(&self, i: usize) -> Point {
        todo!()
    }

    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        todo!()
    }
}
```

**Explanation.**

1. `from_file` has no `self`, so it is an associated function: `Graph::from_file(&file)`
   (Slice 2, Step 4). It takes `&GraphFile`, a borrow. It only copies numbers out of the
   file, so it does not need to own the file, and `main` can still use the file afterwards
   (Slice 1, Step 6).
2. `&self` is a method that **borrows** the graph for reading. Slice 2 used `self` for `Point`,
   because `Point` is `Copy`. A `Graph` is large and is not `Copy`. With `self`, the call
   `graph.node_count()` would move the graph into the method, and you could not use `graph`
   again. With `&self`, any number of methods can read the graph at the same time.
3. `id` and `coord` return their value, not a reference. A `NodeId` and a `Point` are `Copy`,
   so the caller gets a copy.
4. `index_of` returns an `Option<usize>`, because an id may not be in the graph (Slice 1,
   Step 11).
5. The method is called `node_count`, not `len`. A public `len` makes `cargo clippy` ask for
   an `is_empty` too, and `node_count` also says what it counts.

Now the tests. The graph of the Python test file is the shared fixture, with the same
out-of-order ids.

- [ ] Add this at the end of `src/graph.rs`:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::io::{EdgeRecord, NodeRecord};

      const COORDS: [(NodeId, f64, f64); 6] = [
          (50, 0.0, 0.0),
          (2, 4.0, 0.0),
          (17, 0.0, 3.0),
          (8, 3.0, 4.0),
          (33, 20.0, 20.0),
          (7, 20.0, 20.0),
      ];
      const EDGES: [(NodeId, NodeId); 5] = [(50, 2), (50, 17), (17, 8), (2, 33), (33, 7)];

      fn file(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> GraphFile {
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

      fn fixture() -> Graph {
          Graph::from_file(&file(&COORDS, &EDGES)).unwrap()
      }
  ```

  1. `(NodeId, f64, f64)` is a **tuple type**. Like a Python tuple, it groups values. Unlike
     Python, the type of each position is fixed, so `COORDS` is an array of six
     `(u64, f64, f64)` tuples.
  2. `file` is a test helper (Slice 2, Step 5) that turns plain tuples into a `GraphFile`. It
     is like the `graph` fixture in the Python tests. The closure `|&(id, x, y)|` takes the
     tuple out of the reference and gives names to its three parts. It works because the
     tuple contains only `Copy` values.
  3. `collect()` needs a target type (Slice 2, Step 10). You do not write one here. The
     field `nodes` has the type `Vec<NodeRecord>`, so the compiler knows.
  4. `"test".to_string()` makes an owned `String` from a `&str` literal.
  5. `&COORDS` is a reference to an array of six elements. The parameter type `&[(NodeId, f64,
     f64)]` is a **slice**: a borrowed view of any number of elements in a row. Rust converts
     the array reference into a slice for you. Step 7 explains slices.

- [ ] Add four tests inside `mod tests`, after `fixture`:

  ```rust
      #[test]
      fn counts_nodes() {
          let graph = fixture();

          assert_eq!(graph.node_count(), 6);
      }

      #[test]
      fn nodes_keep_file_order() {
          let graph = fixture();

          // position i is the i-th node of the file
          assert_eq!(graph.id(0), 50);
          assert_eq!(graph.id(3), 8);
          assert_eq!(graph.coord(3), Point::new(3.0, 4.0));
      }

      #[test]
      fn index_of_is_the_inverse_of_id() {
          let graph = fixture();

          // every id maps back to its own position
          for i in 0..graph.node_count() {
              assert_eq!(graph.index_of(graph.id(i)), Some(i));
          }
          // an id that is not in the file has no position
          assert_eq!(graph.index_of(999), None);
      }

      #[test]
      fn rejects_repeated_node_id() {
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (1, 5.0, 5.0)];

          let result = Graph::from_file(&file(&nodes, &[]));

          // the error says which id was repeated
          assert_eq!(result.err(), Some(GraphError::DuplicateNode(1)));
      }
  }
  ```

  `result.err()` turns a `Result<Graph, GraphError>` into an `Option<GraphError>`: `Some` of the
  error, or `None` if it was `Ok`. We use it because `unwrap_err()` would need `Graph` to
  implement `Debug`, and we do not want to print a graph with 10^6 nodes.
  `&[]` is an empty slice. `Some(i)` is the `Option` case that holds a value.

### 5. Watch them fail

- [ ] Run `cargo test graph`. Expected: `0 passed; 4 failed`. Each failure says
  `not yet implemented` at the `todo!()` of `from_file`. You also see warnings about unused
  variables in the stubs.

  (`cargo test graph` runs the tests whose path contains `graph`. Slice 2, Step 18 explains
  the filter.)

### 6. Write the node part of `from_file`

**Try first.** Replace the body of `from_file`. Make two empty `Vec`s (`ids` and `coords`) and
a `HashMap` (`index_of`). Loop over `file.nodes` with their positions, and fill all three.
`HashMap::insert(key, value)` returns an `Option` with the **old** value if the key was
already there. Use that to find a repeated id. At the end, return a `Graph` with an
`adjacency` of `n` empty lists.

**Suggested code.**

```rust
    pub fn from_file(file: &GraphFile) -> Result<Graph, GraphError> {
        let n = file.nodes.len();
        let mut ids = Vec::with_capacity(n);
        let mut coords = Vec::with_capacity(n);
        let mut index_of = HashMap::with_capacity(n);

        for (i, node) in file.nodes.iter().enumerate() {
            if index_of.insert(node.id, i).is_some() {
                return Err(GraphError::DuplicateNode(node.id));
            }
            ids.push(node.id);
            coords.push(Point::new(node.x, node.y));
        }

        let adjacency = vec![Vec::new(); n];

        Ok(Graph {
            ids,
            coords,
            adjacency,
            index_of,
        })
    }
```

**Explanation.**

1. `Vec::with_capacity(n)` makes an empty `Vec` and reserves room for `n` elements at once.
   Without it, a `Vec` grows by doubling, and it copies its data at each step. We know `n`, so
   we avoid that work. `HashMap::with_capacity` does the same.
2. These variables have no type written. The compiler infers `Vec<NodeId>` from the
   `ids.push(node.id)` line, and the field types of `Graph` confirm it. `mut` is needed
   because we change them (Slice 0, Step 8).
3. `for (i, node) in file.nodes.iter().enumerate()` is Python's
   `for i, node in enumerate(file.nodes)`. `.iter()` lends each element, so `node` is a
   `&NodeRecord`. Fields of a reference can be read directly: `node.id`.
4. `index_of.insert(node.id, i)` puts the pair in the map. A Python `d[k] = v` returns
   nothing, but `insert` returns the old value as an `Option<usize>`. `.is_some()` is
   `true` for `Some(_)`, so a repeated id returns the error at once. `return` for an early exit
   is the same as in Slice 2, Step 12.
5. `vec![Vec::new(); n]` makes `n` empty lists. It is the `[[] for _ in range(n)]` of Python,
   and not the `[[]] * n`: Rust **clones** the value for each slot, so the lists are
   independent. The element type is inferred in Step 8.
6. `Graph { ids, coords, adjacency, index_of }` uses the short form of Slice 2, Step 4. The
   four variables are **moved** into the struct (Slice 2, Step 3), so the function cannot use
   them after this line, and the graph owns the data when the function returns.

Now the other four methods. Replace the stubs `node_count`, `id`, `coord` and `index_of`:

```rust
    pub fn node_count(&self) -> usize {
        self.ids.len()
    }

    pub fn id(&self, i: usize) -> NodeId {
        self.ids[i]
    }

    pub fn coord(&self, i: usize) -> Point {
        self.coords[i]
    }

    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.index_of.get(&id).copied()
    }
```

1. `self.ids[i]` reads one element. A `NodeId` is `Copy`, so the caller gets a copy. If `i` is
   out of range, the program **panics**. That is the policy of decision D3: a bad position is a
   bug in the caller, not an error in the input. Step 10 tests this.
2. `self.index_of.get(&id)` looks up the key. `get` takes a reference to the key. It returns
   `Option<&usize>`, a reference to the number inside the map. `.copied()` turns an
   `Option<&usize>` into an `Option<usize>` by copying the number. The caller then gets a
   plain value, and not a reference that would keep the graph borrowed.
3. A field and a method can have the same name (`index_of`). `self.index_of` is the field.
   `self.index_of(id)` is the method.

- [ ] Run `cargo test graph`. Expected: `4 passed; 0 failed`. There is one warning left: the
  field `adjacency` is never read.

### 7. Edge tests

Now the neighbours. First add two more stubs inside `impl Graph`, and then the tests.

- [ ] Add after `node_count`:

  ```rust
      pub fn edge_count(&self) -> usize {
          todo!()
      }
  ```

- [ ] Add after `coord`:

  ```rust
      pub fn neighbors(&self, i: usize) -> &[usize] {
          todo!()
      }
  ```

  `&[usize]` is a **slice**: a borrowed view of `usize` values that sit next to each other in
  memory. It is a pointer and a length. The method returns a view into the graph's own list, so
  nothing is copied. A caller that needs its own list can call `.to_vec()`. The code of the
  search calls `neighbors` very often, so a copy at each call would be expensive.

  The signature has one more detail. A function that returns a reference must say where the
  reference comes from. Here the only reference that goes in is `&self`, so Rust fills in the
  rest: the returned slice lives no longer than the borrow of the graph. Rust calls this
  **lifetime elision**. Step 9 shows what it protects you from.

- [ ] Add five tests at the end of `mod tests`:

  ```rust
      #[test]
      fn counts_edges() {
          let graph = fixture();

          assert_eq!(graph.edge_count(), 5);
      }

      #[test]
      fn neighbors_are_symmetric_and_sorted() {
          let graph = fixture();

          // each edge shows in the lists of both ends, in position order
          assert_eq!(graph.neighbors(0), &[1, 2]);
          assert_eq!(graph.neighbors(1), &[0, 4]);
          assert_eq!(graph.neighbors(5), &[4]);
      }

      #[test]
      fn repeated_edges_count_once() {
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)];
          let edges = [(1, 2), (1, 3), (2, 1)];

          let graph = Graph::from_file(&file(&nodes, &edges)).unwrap();

          // (2, 1) is the first edge written backwards
          assert_eq!(graph.edge_count(), 2);
          assert_eq!(graph.neighbors(0), &[1, 2]);
      }

      #[test]
      fn self_loops_are_dropped() {
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0)];
          let edges = [(1, 1), (1, 2)];

          let graph = Graph::from_file(&file(&nodes, &edges)).unwrap();

          // only the edge between two different nodes stays
          assert_eq!(graph.edge_count(), 1);
          assert_eq!(graph.neighbors(0), &[1]);
      }

      #[test]
      fn rejects_edge_to_missing_node() {
          let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0)];
          let edges = [(1, 2), (2, 3)];

          let result = Graph::from_file(&file(&nodes, &edges));

          // the error says which id has no node
          assert_eq!(result.err(), Some(GraphError::UnknownNode(3)));
      }
  ```

  `assert_eq!(graph.neighbors(0), &[1, 2])` compares a slice with a reference to an array.
  Rust allows this, and it reads like the Python `assert neighbors == [1, 2]`.

- [ ] Run `cargo test graph`. Expected: `4 passed; 5 failed`. Four failures say
  `not yet implemented`. The fifth is `rejects_edge_to_missing_node`, because `from_file` does
  not look at the edges yet:

  ```text
  assertion `left == right` failed
    left: None
   right: Some(UnknownNode(3))
  ```

### 8. Write the edge part of `from_file`

**Try first.** In `from_file`, replace the line `let adjacency = vec![Vec::new(); n];` with a
loop over `file.edges`. For each edge, find the positions of its two ends. If an id is not in
`index_of`, return `GraphError::UnknownNode`. Push each end into the other's list. Then write
the bodies of `edge_count` and `neighbors`.

**Suggested code.** Replace the `adjacency` line with:

```rust
        let lookup = |id: NodeId| {
            index_of
                .get(&id)
                .copied()
                .ok_or(GraphError::UnknownNode(id))
        };

        let mut adjacency = vec![Vec::new(); n];
        for edge in &file.edges {
            let a = lookup(edge.source)?;
            let b = lookup(edge.target)?;
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
```

And replace the two stubs:

```rust
    pub fn edge_count(&self) -> usize {
        self.adjacency.iter().map(Vec::len).sum::<usize>() / 2
    }

    pub fn neighbors(&self, i: usize) -> &[usize] {
        &self.adjacency[i]
    }
```

**Explanation.**

1. `lookup` is a **closure** (Slice 2, Step 10) that uses a variable from the function around
   it: `index_of`. As in Python, the closure captures the variable. Unlike Python, the
   compiler checks the borrow: while `lookup` exists, `index_of` is borrowed for reading, so
   you cannot change it or move it. The borrow ends after the last use of `lookup`. That is
   why the `Ok(Graph { ..., index_of })` at the end of the function is still allowed.
2. `.ok_or(GraphError::UnknownNode(id))` turns an `Option` into a `Result` (Slice 1, Step 14):
   `Some(position)` becomes `Ok(position)`, and `None` becomes our error.
3. `lookup(edge.source)?` is the `?` of Slice 1, Step 13: on `Err`, `from_file` returns that
   error at once. Both sides have the type `GraphError`, so no conversion is needed.
4. `for edge in &file.edges` loops over references to the edges. It is the same as
   `file.edges.iter()`. Without the `&`, the loop would try to move the `Vec` out of the
   file, and the compiler would refuse, because `file` is only borrowed.
5. `adjacency` needs `mut` now, because `push` changes the inner lists. Its type
   (`Vec<Vec<usize>>`) is inferred from the pushes.
6. In `edge_count`, each edge appears in two lists, so the sum of the list lengths is twice the
   number of edges. `Vec::len` is a function that we pass to `map` by its name, like
   `map(len, lists)` in Python. `sum::<usize>()` names the type of the sum, with the
   "turbofish" `::<...>`. The same problem as in `collect`: `sum` can add up many types, and it
   cannot guess which one you want.
7. `&self.adjacency[i]` has the type `&Vec<usize>`. The signature says `&[usize]`, and Rust
   converts a reference to a `Vec` into a slice. The slice borrows from `self`, as the
   elided lifetime says.

- [ ] Run `cargo test graph`. Expected: `7 passed; 2 failed`.

  ```text
  ---- graph::tests::repeated_edges_count_once stdout ----
    left: 3
   right: 2
  ---- graph::tests::self_loops_are_dropped stdout ----
    left: 2
   right: 1
  ```

  The edge count is wrong when an edge is repeated, and also with a self-loop. Fix the
  self-loop first.

- [ ] Add these lines inside the `for edge` loop, after the two `lookup` lines:

  ```rust
            if a == b {
                continue;
            }
  ```

  `continue` skips to the next edge, as in Python.

Now remove the repeated neighbours. A `Vec` has the method `dedup`, which removes repeated
values. Try it alone first.

- [ ] Add this after the loop over the edges:

  ```rust
        for neighbors in &mut adjacency {
            neighbors.dedup();
        }
  ```

  `&mut adjacency` lends the outer `Vec` for **changing**, so each `neighbors` is a `&mut
  Vec<usize>`. This is the main rule of borrowing: at any moment a value has either many
  readers or one writer, and never both. The compiler checks this rule, and that is how Rust
  finds a whole class of bugs (for example, changing a list while you loop over it) before
  the program runs.

- [ ] Run `cargo test graph`. Expected: `8 passed; 1 failed`.

  ```text
  ---- graph::tests::repeated_edges_count_once stdout ----
    left: [1, 2, 1]
   right: [1, 2]
  ```

  `dedup` removes a value only when it sits **next to** an equal value. The list of node 0 is
  `[1, 2, 1]`: the second `1` is not next to the first. Sort the list first, and then equal
  values are always neighbours.

- [ ] Add one line before `neighbors.dedup();`:

  ```rust
            neighbors.sort_unstable();
  ```

  `sort_unstable` is faster than `sort`. The only difference is that it may reorder equal
  elements, and equal `usize` values cannot be told apart. A sorted list has a second benefit:
  every list has a fixed order, so later results do not depend on the order of the file.

- [ ] Run `cargo test graph`. Expected: `9 passed; 0 failed`.

### 9. Experiment: a view cannot outlive its owner

- [ ] Add this temporary test at the end of `mod tests`:

  ```rust
      #[test]
      fn experiment() {
          let graph = fixture();
          let near = graph.neighbors(0);
          drop(graph);
          println!("{near:?}");
      }
  ```

- [ ] Run `cargo test graph`. Expected: the build fails.

  ```text
  error[E0505]: cannot move out of `graph` because it is borrowed
      |
  222 |         let graph = fixture();
      |             ----- binding `graph` declared here
  223 |         let near = graph.neighbors(0);
      |                    ----- borrow of `graph` occurs here
  224 |         drop(graph);
      |              ^^^^^ move out of `graph` occurs here
  225 |         println!("{near:?}");
      |                    ---- borrow later used here
  ```

  (The line numbers are different on your computer.)

`near` is a view into the memory of `graph`. `drop(graph)` frees that memory early. If this
compiled, the next line would read freed memory. Python avoids the problem with reference
counting: the list would stay alive while `near` points to it. Rust has no counter. The
compiler proves that every view ends before its owner does, and the proof costs nothing when
the program runs.

- [ ] Delete the `experiment` test.

### 10. Port `TestDistanceFromNode`

The Python tests check `_distance_from_node` on the same six-node graph. The Rust method
returns **positions**, not ids, so the tests convert with `graph.id(i)` when they need an id.

| Python test | Rust test |
|---|---|
| `test_excludes_anchor` | `distance_excludes_anchor` |
| `test_covers_all_other_nodes` | `distance_covers_all_other_nodes` |
| `test_distances_match_coordinates` | `distances_match_coordinates` |
| `test_sorted_nearest_first` | `distance_sorted_nearest_first` |
| `test_known_distances` | `distance_known_values` |
| `test_coincident_node_is_zero` | `coincident_node_has_zero_distance` |
| `test_single_node_graph_returns_empty` | `single_node_has_no_distances` |

I added three more tests: two for `radius_from`, and one that checks the panic of Step 6.

- [ ] Add the stubs inside `impl Graph`, after `index_of`:

  ```rust
      pub fn distance_from_node(&self, anchor: usize) -> Vec<(usize, f64)> {
          todo!()
      }

      pub fn radius_from(&self, anchor: usize) -> f64 {
          todo!()
      }
  ```

  The return type `Vec<(usize, f64)>` is a list of `(position, distance)` pairs, like the
  `list[tuple[int, float]]` of the Python function.

- [ ] Add the helper `assert_close` from Slice 2, Step 5 to `mod tests`, after the `use` lines:

  ```rust
      fn assert_close(actual: f64, expected: f64) {
          assert!(
              (actual - expected).abs() < 1e-9,
              "expected {expected}, got {actual}"
          );
      }
  ```

- [ ] Add the tests at the end of `mod tests`:

  ```rust
      fn anchor_50(graph: &Graph) -> usize {
          graph.index_of(50).unwrap()
      }

      #[test]
      fn distance_excludes_anchor() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let result = graph.distance_from_node(anchor);

          // the anchor is not among its own distances
          assert!(result.iter().all(|&(i, _)| i != anchor));
      }

      #[test]
      fn distance_covers_all_other_nodes() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let result = graph.distance_from_node(anchor);

          // every other node appears exactly once
          let mut ids: Vec<NodeId> = result.iter().map(|&(i, _)| graph.id(i)).collect();
          ids.sort();
          assert_eq!(ids, vec![2, 7, 8, 17, 33]);
      }

      #[test]
      fn distances_match_coordinates() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let result = graph.distance_from_node(anchor);

          // each distance matches its node (anchor 50 is at the origin)
          for (i, dist) in result {
              let (_, x, y) = COORDS[i];
              assert_close(dist, x.hypot(y));
          }
      }

      #[test]
      fn distance_sorted_nearest_first() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let result = graph.distance_from_node(anchor);

          // distances come back in non-decreasing order
          assert!(result.windows(2).all(|w| w[0].1 <= w[1].1));
      }

      #[test]
      fn distance_known_values() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let result = graph.distance_from_node(anchor);

          // hand-computed order and values (33 and 7 tie, file order wins)
          let ids: Vec<NodeId> = result.iter().map(|&(i, _)| graph.id(i)).collect();
          assert_eq!(ids, vec![17, 2, 8, 33, 7]);
          let far = 20.0_f64.hypot(20.0);
          let expected = [3.0, 4.0, 5.0, far, far];
          for (&(_, dist), want) in result.iter().zip(expected) {
              assert_close(dist, want);
          }
      }

      #[test]
      fn coincident_node_has_zero_distance() {
          let graph = fixture();
          let anchor = graph.index_of(33).unwrap();
          let twin = graph.index_of(7).unwrap();

          let result = graph.distance_from_node(anchor);

          // node 7 shares the anchor's coordinates, so it is kept at 0
          let dist = result.iter().find(|&&(i, _)| i == twin).unwrap().1;
          assert_close(dist, 0.0);
      }

      #[test]
      fn single_node_has_no_distances() {
          let graph = Graph::from_file(&file(&[(50, 0.0, 0.0)], &[])).unwrap();

          let result = graph.distance_from_node(0);

          // a lone anchor has no other nodes to measure
          assert!(result.is_empty());
      }

      #[test]
      fn radius_is_the_largest_distance() {
          let graph = fixture();
          let anchor = anchor_50(&graph);

          let radius = graph.radius_from(anchor);

          // node 33 is the farthest from node 50
          assert_close(radius, 20.0_f64.hypot(20.0));
      }

      #[test]
      fn radius_of_single_node_is_zero() {
          let graph = Graph::from_file(&file(&[(50, 0.0, 0.0)], &[])).unwrap();

          let radius = graph.radius_from(0);

          // no other node, so nothing is farther than the anchor itself
          assert_close(radius, 0.0);
      }

      #[test]
      #[should_panic(expected = "index out of bounds")]
      fn distance_from_missing_index_panics() {
          let graph = fixture();

          // the fixture has nodes 0..=5, so 6 is a bug in the caller
          graph.distance_from_node(6);
      }
  ```

  Notes on the new test code:

  1. `|&(i, _)|` takes the pair out of the reference. `_` means "I do not need this part".
     `all(...)` is the Python `all(...)`.
  2. `ids.sort()` works on `Vec<u64>`. Integers have a full order, so no extra argument is
     needed. Step 11 shows what is different for floats.
  3. `for (i, dist) in result` loops over the `Vec` itself, so the loop takes the pairs out of
     it. We do not need `result` afterwards. `COORDS[i]` works because the position of a
     node is its place in the file, and the file is built from `COORDS` in order.
     `x.hypot(y)` is Python's `math.hypot(x, y)`.
  4. `windows(2)` gives every pair of neighbours in a slice, like `zip(l, l[1:])`. `w[0].1` is
     the second part of the first pair: tuple parts are read with `.0`, `.1`, and so on.
  5. `result.iter().zip(expected)` pairs each result with the expected value. The pattern
     `(&(_, dist), want)` takes the distance out of the reference.
  6. `find` gives its closure a reference to each item, and `iter` already gives references,
     so the pattern has two `&`: `|&&(i, _)|`.
  7. `#[should_panic(expected = "...")]` makes the test pass only if the code panics **and**
     the message contains the text. Without `expected`, a stub with `todo!()` would pass this
     test, for the wrong reason.

- [ ] Run `cargo test graph`. Expected: `9 passed; 10 failed`. The new tests fail with
  `not yet implemented`. The `should_panic` test also fails, because the message of `todo!()`
  does not contain `index out of bounds`.

### 11. Write `distance_from_node`

**Try first.** The Python function loops over the nodes, skips the anchor, collects
`(node_id, distance)` pairs and sorts them by distance. Write that with positions: take the
anchor's coordinates, build the list of `(position, distance)` pairs for every other position,
and return it unsorted for now.

**Suggested code.**

```rust
    pub fn distance_from_node(&self, anchor: usize) -> Vec<(usize, f64)> {
        let origin = self.coords[anchor];
        let distances: Vec<(usize, f64)> = (0..self.node_count())
            .filter(|&i| i != anchor)
            .map(|i| (i, origin.dist(self.coords[i])))
            .collect();
        distances
    }
```

**Explanation.**

1. `0..self.node_count()` is a **range**, the `range(n)` of Python. It is an iterator over the
   positions `0, 1, ..., n - 1`.
2. `.filter(|&i| i != anchor)` keeps the items for which the closure gives `true`. It is the
   `continue` of the Python loop. `filter` lends each item to the closure, so the pattern is
   `|&i|`, as in Slice 2, Step 10.
3. `.map(|i| (i, origin.dist(self.coords[i])))` makes a tuple of the position and the distance.
   `Point::dist` is the `_euclidean_distance` of Python. It is from Slice 2, and it takes two
   `Point`s, not two dicts.
4. `.collect()` builds the `Vec`. The type on the `let` line tells it what to build (Slice 2,
   Step 10).

- [ ] Run `cargo test graph`. Expected: `15 passed; 4 failed`. The four failures are the two
  `radius` tests (still `todo!()`), `distance_sorted_nearest_first` and
  `distance_known_values`.

Now sort. The Python code uses `distances.sort(key=lambda pair: pair[1])`. Try the same idea:

- [ ] Change `let distances` to `let mut distances` (sorting changes the list in place), and
  add this line before the last line:

  ```rust
        distances.sort_by_key(|&(_, d)| d);
  ```

- [ ] Run `cargo build`. Expected: the build fails.

  ```text
  error[E0277]: the trait bound `f64: Ord` is not satisfied
      |
  109 |         distances.sort_by_key(|&(_, d)| d);
      |                   ^^^^^^^^^^^ the trait `Ord` is not implemented for `f64`
  ```

`Ord` is the trait for a **total order**: for any two values, one is smaller, or they are
equal. `f64` does not have it, because of `NaN`. `NaN < 1.0` and `NaN > 1.0` are both
`false`. Python sorts a list with `NaN` and gives a result with no meaning, and it says
nothing. Rust does not sort floats until you choose what to do with `NaN`. (`ids.sort()` in
Step 10 worked, because integers are `Ord`.)

- [ ] Replace the `sort_by_key` line with:

  ```rust
        distances.sort_by(|a, b| a.1.total_cmp(&b.1));
  ```

`sort_by` takes a closure that compares two elements. Here `a` and `b` are references to two
pairs, and `.1` is the distance. `total_cmp` is a method of `f64` that gives a full order. It
gives `NaN` a fixed place at the end of the order. Our distances only contain `NaN` if a file
has a coordinate that is `NaN`, and JSON cannot write one.

Both `sort` and `sort_by` are **stable**, as `sorted` in Python is: elements that compare equal
keep their order. That is what `distance_known_values` needs: nodes 33 and 7 have the same
distance, and node 33 comes first in the file. (`sort_unstable_by` does not promise this.)

- [ ] Run `cargo test graph`. Expected: `17 passed; 2 failed`. The two failures are the
  `radius` tests.

### 12. Write `radius_from`

The Python function is `max(d for _, d in distances)`. For a query with one node, `max` of an
empty sequence raises `ValueError`. A radius of `0.0` is the correct answer for a graph with
one node, so the Rust method returns `0.0` and does not fail. This is the only place where the
port is different from Python on purpose.

**Try first.** Write `radius_from` with `distance_from_node`, `.iter()`, `.map(...)` to take
the distance out of each pair, and `fold`. `fold(start, f)` is `functools.reduce(f, items,
start)`. The function `f64::max` takes two `f64` and gives the larger one.

**Suggested code.**

```rust
    pub fn radius_from(&self, anchor: usize) -> f64 {
        self.distance_from_node(anchor)
            .iter()
            .map(|&(_, d)| d)
            .fold(0.0, f64::max)
    }
```

**Explanation.**

1. `.fold(0.0, f64::max)` starts with `0.0` and keeps the larger of the current value and the
   next distance. A distance is never negative, so `0.0` is a safe start. For an empty list,
   the result is the start value.
2. Why not `.max()`? An iterator's `max` needs `Ord`, so it has the same problem with `f64` as
   `sort_by_key`. `f64::max` is a method of the float itself, and it takes two floats.
3. `f64::max` is passed by name, without `()`, like `Vec::len` in Step 8.
4. This sorts the list before it takes the maximum, which is more work than needed. We call it
   once for each search, on a graph with fewer than 20 nodes, so it does not matter.

- [ ] Run `cargo test graph`. Expected: `19 passed; 0 failed`.

### 13. Use it from `main`

- [ ] Replace all of `src/main.rs`:

  ```rust
  use std::error::Error;

  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;

  fn main() -> Result<(), Box<dyn Error>> {
      let path = std::env::args()
          .nth(1)
          .ok_or("usage: gsearch <graph.json>")?;

      let file = read_graph_file(&path)?;
      let graph = Graph::from_file(&file)?;

      println!(
          "{}: {} nodes, {} edges",
          file.graph_id,
          graph.node_count(),
          graph.edge_count(),
      );
      println!("anchor radius: {:.4}", graph.radius_from(0));
      Ok(())
  }
  ```

1. `Graph::from_file(&file)?` returns a `GraphError` on bad input. `main` returns
   `Box<dyn Error>`, and `?` converts the error into that type. This works because
   `GraphError` implements `Error` (Step 3, point 5), so the `impl Error` line has a use now.
2. `file.graph_id` is still available, because `from_file` only borrowed `file`.
3. `{:.4}` prints a float with four decimals, like `:.4f` in Python.
4. `graph.radius_from(0)` uses position `0`, which is the **first node of the file**. That is
   the anchor of `Q` in Python (`_get_anchor_node`). With positions, "the first node" is simply
   `0`. This is the radius of the query, `Q_radius`, in `run_gsearch`.

- [ ] Experiment. Add the line `println!("{}", graph.ids.len());` before `Ok(())`, and run
  `cargo build`. Expected:

  ```text
  error[E0616]: field `ids` of struct `Graph` is private
    |
  4 |     println!("{}", graph.ids.len());
    |                          ^^^ private field
  ```

  The compiler enforces the privacy of Step 2. Remove the line.

- [ ] Run:

  ```powershell
  cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
  cargo run -- ..\gsearch\data\db\graphs\gg00041.json
  ```

  Expected:

  ```text
  gd00000_q: 3 nodes, 3 edges
  anchor radius: 5.7793
  ```

  ```text
  gg00041: 78 nodes, 114 edges
  anchor radius: 1327.0393
  ```

- [ ] Compare with Python. The check computes the distance from the first node to the farthest
  node, with no networkx:

  ```powershell
  python -c "import json, math; d = json.load(open('../gsearch/data/db/graphs/gg00041.json')); a = d['nodes'][0]; print(max(math.hypot(n['x'] - a['x'], n['y'] - a['y']) for n in d['nodes'][1:]))"
  ```

  Expected: `1327.039295531232`. Rust printed the same number, rounded to four decimals.

### 14. Time it

- [ ] Replace all of `src/main.rs` with this version, which prints the time of each part:

  ```rust
  use std::error::Error;
  use std::time::Instant;

  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;

  fn main() -> Result<(), Box<dyn Error>> {
      let path = std::env::args()
          .nth(1)
          .ok_or("usage: gsearch <graph.json>")?;

      let start = Instant::now();
      let file = read_graph_file(&path)?;
      eprintln!("read file:   {:.3} s", start.elapsed().as_secs_f64());

      let start = Instant::now();
      let graph = Graph::from_file(&file)?;
      eprintln!("build graph: {:.3} s", start.elapsed().as_secs_f64());

      println!(
          "{}: {} nodes, {} edges",
          file.graph_id,
          graph.node_count(),
          graph.edge_count(),
      );
      println!("anchor radius: {:.4}", graph.radius_from(0));
      Ok(())
  }
  ```

  `Instant::now()` reads the clock, and `.elapsed()` gives the time since then. This is
  `time.perf_counter()` of Python. `eprintln!` writes to the error stream, so that the real
  output stays clean when we print JSON results in Slice 8.

- [ ] Build in release mode and run the large file:

  ```powershell
  cargo build --release
  .\target\release\gsearch.exe ..\gsearch\data\db\graphs\go00001.json
  ```

  Expected (your times will be a little different):

  ```text
  read file:   0.35 s
  build graph: 0.70 s
  go00001: 452628 nodes, 5604320 edges
  anchor radius: 1277.7344
  ```

  The edge count is the same as the number of edges in the file, because this file has no
  repeated edges and no self-loops.

- [ ] Run the debug build for comparison: `cargo run -q -- ..\gsearch\data\db\graphs\go00001.json`.
  Expected: about `3.4 s` to read and `2.5 s` to build.

- [ ] Time the Python loader from the `gsearch` folder:

  ```powershell
  cd ..\gsearch
  Measure-Command { uv run python -c "from gsearch.io import load_graph; load_graph('data/db/graphs/go00001.json')" } | Select-Object TotalSeconds
  cd ..\gsearch_rs
  ```

  Expected: about 19 seconds. This includes the start of `uv` and the import of `networkx`.
  Rust does the same job in about one second.

Building the graph takes longer than reading the file. The builder makes about 11 million hash
lookups, one for each end of each edge, and the same number of `push` calls on 452 628
separate lists. Slice 9 measures which part is slow and speeds it up. For now, a full second
for the biggest graph is fine.

### 15. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
- [ ] Run `cargo test`. Expected: all tests pass. The `graph` tests are 19 of them, with the
  tests of Slices 1 and 2.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 3: graph type, distance_from_node, radius"
  ```

---

## What comes next

Slice 4 ports `spatial_graph_index.py`: `query_radius` (all nodes within a distance of a point,
nearest first) and `make_crop` (the part of the graph around an anchor that is connected to
it). First decide D6, the spatial index: a KD-tree from the `kiddo` crate, or a grid that you
write yourself. `make_crop` builds a new, smaller `Graph`, so Slice 4 adds a second way to
make one. It uses `coord`, `neighbors` and `index_of` from this slice.

Slice 8 chooses the anchor in `run_gsearch`. It is position `0`, and it must handle a query
file with no nodes. Python raises `StopIteration` in that case.
