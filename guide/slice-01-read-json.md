# Slice 1: Read a graph file

**Goal.** Port `io.py`: read a graph JSON file into Rust data, and print a summary from the
command line. This slice reads the file into structs that match the file format. It does not
build a graph yet, because the graph type is decision D5, which you make at the end of this
slice.

**Outcome.**

```text
> cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
gd00000_q: 3 nodes, 3 edges
first node: NodeRecord { id: 8018, x: -2.620017385197598, y: -1.7181254610026941 }
```

A release build reads the 238 MB `go00001.json` in less than a second. Python's `json.load`
takes about 3 seconds.

**New ideas.** Crates and features; library vs binary crate; modules (`mod`, `pub`, `use`);
structs; fixed-size number types; `String` and `Vec`; type aliases; `#[derive]`; `serde`;
`&str` and borrowing; `Result`; `todo!()`; unit tests; `Option`; panics vs errors; the `?`
operator; `Box<dyn Error>`; release builds.

**Starting point.** Slice 0 is done: `cargo run` prints `Hello, gsearch!`.

---

## Steps

### 1. Add the JSON crates

- [ ] Run:

  ```powershell
  cargo add serde --features derive
  cargo add serde_json
  ```

  Expected: `Adding serde v1.0.x to dependencies` with `+ derive` under `Features`, then
  `Adding serde_json v1.0.x to dependencies`.

- [ ] Open `Cargo.toml`. The `[dependencies]` section now contains:

  ```toml
  serde = { version = "1.0.229", features = ["derive"] }
  serde_json = "1.0.151"
  ```

A **crate** is a Rust package. It comes from <https://crates.io>, like PyPI. A **feature** is an
optional part of a crate that you turn on, like `pip install "pkg[extra]"`. `serde` is the
general serialization framework. Its `derive` feature writes the parsing code for your structs
(Step 4). `serde_json` is the JSON format for `serde`. Cargo also wrote `Cargo.lock`, which
records exact versions, like `uv.lock`. Commit it.

### 2. Make a library crate

Decision D1 puts the real code in a library and keeps `main.rs` thin, like `cli.py`.

- [ ] Make `src/lib.rs` with one line:

  ```rust
  pub mod io;
  ```

- [ ] Make an empty file `src/io.rs`.

Cargo finds `src/lib.rs` and builds a library crate called `gsearch`. Cargo also builds
`src/main.rs` as a binary crate that uses the library. Tests, and later the Python bindings,
use the library without the CLI.

In Python, any `.py` file in the package can be imported. In Rust, a file is only part of the
crate if a parent module declares it with `mod`. `pub mod io;` means "there is a module
`io`, its code is in `io.rs`, and code outside this crate can use it".

`pub` is needed because in Rust everything is **private by default**. Python uses a leading
`_` as a convention. Rust makes the compiler enforce privacy. `crate::io` (our module) and
`std::io` (the standard library) do not conflict, because each has a full path.

### 3. Structs for the file format

The files look like this:

```json
{"graph_id": "gd00000_q",
 "nodes": [{"id": 8018, "x": -2.62, "y": -1.71}, ...],
 "edges": [{"source": 8018, "target": 3965}, ...]}
```

**Try first.** In `src/io.rs`, write three structs:

- `NodeRecord` with `id`, `x`, `y`.
- `EdgeRecord` with `source`, `target`.
- `GraphFile` with `graph_id`, `nodes` (a list of `NodeRecord`) and `edges` (a list of
  `EdgeRecord`).

Use `u64` for ids, `f64` for coordinates, `String` for text, and `Vec<T>` for a list of `T`.
Make the structs and their fields `pub`. A struct is like a `@dataclass`: named fields with
types, and no methods yet.

**Suggested code.**

```rust
/// A node id as written in the JSON files.
pub type NodeId = u64;

pub struct NodeRecord {
    pub id: NodeId,
    pub x: f64,
    pub y: f64,
}

pub struct EdgeRecord {
    pub source: NodeId,
    pub target: NodeId,
}

pub struct GraphFile {
    pub graph_id: String,
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<EdgeRecord>,
}
```

**Explanation.**

1. `pub type NodeId = u64;` is a **type alias**, like `NodeId: TypeAlias = int`. It does not
   make a new type. It gives the meaning a name and lets you change the type in one place.
   `///` starts a doc comment. rust-analyzer shows it when you hover.
2. Python has one `int` with no size limit. Rust has fixed sizes: `u8` to `u64` are unsigned,
   and `i8` to `i64` are signed. `u64` holds 0 to about 1.8 × 10^19. The largest query id in
   the data is 616035, so this is safe. If a file contains `-1`, parsing fails with an error.
   It does not silently accept a bad id.
3. `f64` is the same 64-bit float as Python's `float` and numpy's `float64`.
4. `String` is text that the struct **owns**. When a `GraphFile` is dropped, its `String`
   and its `Vec`s are freed with it. `Vec<NodeRecord>` is a growable list, like
   `list[NodeRecord]`, but every element has the same type. The elements are stored next to
   each other in memory, not as pointers to separate objects. This is part of why Rust uses
   much less memory than networkx for a large graph.
5. The fields are separated by commas, and there is no semicolon after the `}` of a struct.

### 4. Derive `Debug` and `Deserialize`

- [ ] Add this line at the top of `src/io.rs`:

  ```rust
  use serde::Deserialize;
  ```

- [ ] Put this line directly above each of the three `pub struct` lines:

  ```rust
  #[derive(Debug, Deserialize)]
  ```

`#[derive(...)]` asks the compiler to write code for the struct, like `@dataclass` writes
`__init__` and `__repr__`. `Debug` writes a printable form for `{:?}` (Step 15).
`Deserialize` comes from `serde`. It writes code that builds the struct from JSON, and the
struct's field names must be the same as the JSON keys. Unknown keys in the file are ignored.

`use serde::Deserialize;` brings the name into scope, like `from serde import Deserialize`.

### 5. Build

- [ ] Run `cargo build`. Expected: `Finished` with no errors. `main.rs` does not use the
  library yet, so nothing changes when you run it.

### 6. A function stub

**Try first.** Below the structs, write a function `parse_graph` that takes the JSON text and
returns a `Result` that holds a `GraphFile` or a `serde_json::Error`. Make the body
`todo!()`.

**Suggested code.**

```rust
/// Parse the JSON text of a graph file.
pub fn parse_graph(json: &str) -> Result<GraphFile, serde_json::Error> {
    todo!()
}
```

**Explanation.**

1. `json: &str` means "a borrowed view of some text". The `&` makes it a **reference**. The
   function can read the text, but it does not own it, so the caller keeps its `String`.
   This is your first contact with **ownership**. In Python, every object can be shared by any
   number of names, and the garbage collector frees it later. In Rust, each value has one owner.
   Other code can borrow it with `&`, and the compiler checks that no borrow lives longer than
   the owner. You already know references from Python. The new part is that the compiler
   checks them.
2. `&str` vs `String`: `String` owns its text and can grow. `&str` only looks at text that
   something else owns. Function arguments that only read text are almost always `&str`.
3. `Result<GraphFile, serde_json::Error>` is the return type. A `Result` is either
   `Ok(value)` or `Err(error)`. Rust has no exceptions. When a function can fail, its
   return type says so, and the caller must deal with the `Err` case. You can think of it as a
   return type of `GraphFile | ParseError` that the type checker makes you handle.
4. `todo!()` is a macro that compiles anywhere and **panics** (crashes with a message) when it
   runs. With it, you can write the tests first and watch them fail.

- [ ] Run `cargo build`. Expected: it builds, with
  `warning: unused variable: `json``. That is fine for now.

### 7. Write the tests first

Rust unit tests usually go at the bottom of the same file as the code.

- [ ] Add this at the end of `src/io.rs`:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      const TRIANGLE: &str = r#"{
          "graph_id": "tri",
          "nodes": [
              {"id": 10, "x": 0.0, "y": 0.0},
              {"id": 20, "x": 3.0, "y": 0.0},
              {"id": 30, "x": 0.0, "y": 4.0}
          ],
          "edges": [
              {"source": 10, "target": 20},
              {"source": 20, "target": 30}
          ]
      }"#;
  ```

1. `#[cfg(test)]` means "compile the next item only for `cargo test`". The tests are not in
   the real program.
2. `mod tests { ... }` is a module written inside the file, not in a separate file.
   `use super::*;` imports everything from the parent module (`io`), which also includes the
   private items. Tests can therefore test private functions, as your pytest files do with
   `_make_annulus_mask`.
3. `r#"..."#` is a **raw string**. Backslashes and `"` inside it have no special meaning, so
   JSON can be pasted in unchanged, like Python's `r"..."`.
4. `const TRIANGLE: &str` is a constant. A string literal is a `&str` that borrows text
   stored in the program file itself.

- [ ] Continue inside `mod tests` with the first test:

  ```rust
      #[test]
      fn parses_nodes_and_edges() {
          let graph = parse_graph(TRIANGLE).unwrap();

          // every node and edge in the file is read
          assert_eq!(graph.graph_id, "tri");
          assert_eq!(graph.nodes.len(), 3);
          assert_eq!(graph.edges.len(), 2);
          // fields land in the right struct members
          assert_eq!(graph.nodes[1].id, 20);
          assert_eq!(graph.nodes[2].y, 4.0);
          assert_eq!(graph.edges[1].target, 30);
      }
  ```

`#[test]` marks a test function, like the `test_` prefix in pytest. `.unwrap()` takes the
value out of an `Ok`, or panics if the result is an `Err`. In a test, a panic is a failure,
so `unwrap()` here works like letting an exception propagate in pytest. `assert_eq!(a, b)`
prints both values when it fails.

- [ ] Add the second test, and close the module with `}`:

  ```rust
      #[test]
      fn rejects_node_without_coordinates() {
          let json = r#"{"graph_id": "bad", "nodes": [{"id": 1, "x": 0.0}], "edges": []}"#;

          let result = parse_graph(json);

          // a missing "y" is an error, not a silent default
          assert!(result.is_err());
      }
  }
  ```

### 8. Run the tests and watch them fail

- [ ] Run `cargo test`. Expected:

  ```text
  running 2 tests
  test io::tests::parses_nodes_and_edges ... FAILED
  test io::tests::rejects_node_without_coordinates ... FAILED
  ...
  ---- io::tests::parses_nodes_and_edges stdout ----
  thread 'io::tests::parses_nodes_and_edges' (...) panicked at src\io.rs:...:
  not yet implemented
  ...
  test result: FAILED. 0 passed; 2 failed
  ```

Both tests fail at `todo!()`, so they really call `parse_graph`.

### 9. Write the function

- [ ] Replace `todo!()` with:

  ```rust
      serde_json::from_str(json)
  ```

There is no `return` and no semicolon. In Rust, the last expression in a function body is
its return value. If you add a `;`, the line becomes a statement and returns nothing, and
the compiler reports `mismatched types`. Try it once to see the error, then remove the `;`.

You also do not tell `from_str` to build a `GraphFile`. Rust **infers** that from the return
type of `parse_graph`. Python's `json.loads` always gives dicts and lists. Here, the target
type decides the parsing, and wrong data is found during parsing, not later when code reads
a missing key.

### 10. Run the tests and watch them pass

- [ ] Run `cargo test`. Expected: `test result: ok. 2 passed; 0 failed`.

### 11. A first `main`

**Try first.** Write `main` so that it:
1. takes the first command-line argument as a path,
2. reads the file into a `String` with `std::fs::read_to_string`,
3. calls `parse_graph`,
4. prints `<graph_id>: <n> nodes, <m> edges`.

Use `.expect("message")` for each step that can fail. Arguments come from
`std::env::args()`.

**Suggested code.** Replace all of `src/main.rs`:

```rust
use gsearch::io::parse_graph;

fn main() {
    let path = std::env::args().nth(1).expect("usage: gsearch <graph.json>");
    let text = std::fs::read_to_string(&path).expect("could not read the file");
    let graph = parse_graph(&text).expect("could not parse the JSON");

    println!(
        "{}: {} nodes, {} edges",
        graph.graph_id,
        graph.nodes.len(),
        graph.edges.len()
    );
}
```

**Explanation.**

1. `use gsearch::io::parse_graph;` imports from the library crate. Its name is `gsearch` (from
   `Cargo.toml`), so this is like `from gsearch.io import parse_graph`.
2. `std::env::args()` is an **iterator** over the arguments, like `sys.argv`. Element 0 is the
   program, so `.nth(1)` takes the first real argument. `nth` returns an `Option<String>`.
   An `Option` is either `Some(value)` or `None`. Rust has no `None` that every variable can
   hold. A value that may be missing has the type `Option<T>`, like `str | None`, and you must
   handle the missing case before you can use the value.
3. `.expect("...")` takes the value out of `Some` or `Ok`. If there is no value, it panics with
   your message. It is like `unwrap()` with a better message.
4. `&path` and `&text` lend the strings to the functions. `main` still owns them after each
   call. `parse_graph` wants a `&str`, and `&text` is a `&String`. Rust converts it for you.

### 12. Run it

- [ ] Run (the `--` separates Cargo's own arguments from your program's arguments):

  ```powershell
  cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
  ```

  Expected: `gd00000_q: 3 nodes, 3 edges`

- [ ] Run it on a small target graph:

  ```powershell
  cargo run -- ..\gsearch\data\db\graphs\gg00041.json
  ```

  Expected: `gg00041: 78 nodes, 114 edges`

- [ ] Run it with no argument: `cargo run`. Expected:

  ```text
  thread 'main' (12345) panicked at src\main.rs:4:40:
  usage: gsearch <graph.json>
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  ```

  The number in brackets is a thread id, so yours is different. `4:40` is the line and column
  of the `expect` that panicked.

A panic is Rust's crash. It is the right response to a **bug**, for example an index out of
range, or a broken invariant. A missing argument or a missing file is not a bug. It is a
normal user error, and it should be reported as an error. The next steps do that.

### 13. Return errors with `?`

- [ ] In `src/io.rs`, change the `use` lines at the top to:

  ```rust
  use std::error::Error;
  use std::fs;
  use std::path::Path;

  use serde::Deserialize;
  ```

- [ ] Add this function after `parse_graph`:

  ```rust
  /// Read and parse a graph file from disk.
  pub fn read_graph_file(path: &Path) -> Result<GraphFile, Box<dyn Error>> {
      let text = fs::read_to_string(path)?;
      let graph = parse_graph(&text)?;
      Ok(graph)
  }
  ```

1. The `?` after a call means: "if this is `Err`, return that error from my function now;
   if it is `Ok`, give me the value". It is the Rust version of letting an exception
   propagate, but you can see it in the code, and the function's type says that it can fail.
2. These two calls fail with different error types: `std::io::Error` and
   `serde_json::Error`. `Box<dyn Error>` means "any type that implements the `Error`
   trait, stored on the heap". It is like a type hint of `Exception`. `?` converts each
   error into the box. A **trait** is a set of methods that a type promises to have (Slice 2
   explains traits more). Storing the value "on the heap" means in memory that is
   allocated at run time. We need this because the two error types have different sizes.
3. `&Path` is a borrowed file path, like a `pathlib.Path` that you only read. Its owned form is
   `PathBuf` (Step 14). This is the same pairing as `&str` and `String`.
4. `Ok(graph)` wraps the value in the success case. We do not use `parse_graph(&text)` as the
   last line, because its error type is `serde_json::Error`, not `Box<dyn Error>`. `?` does the
   conversion, and then `Ok` wraps the result again.

We read the whole file into a `String` first and parse it after. This is faster in
`serde_json` than parsing from an open file, because the parser can then look ahead freely.

### 14. A `main` that returns errors

- [ ] Replace all of `src/main.rs`:

  ```rust
  use std::error::Error;
  use std::path::PathBuf;

  use gsearch::io::read_graph_file;

  fn main() -> Result<(), Box<dyn Error>> {
      let path: PathBuf = std::env::args()
          .nth(1)
          .ok_or("usage: gsearch <graph.json>")?
          .into();

      let graph = read_graph_file(&path)?;

      println!(
          "{}: {} nodes, {} edges",
          graph.graph_id,
          graph.nodes.len(),
          graph.edges.len()
      );
      println!("first node: {:?}", graph.nodes[0]);
      Ok(())
  }
  ```

1. `main` can return a `Result`. If it returns `Err`, Rust prints the error and exits with
   code 1. `()` is the "unit" type, which means "no value", like a function that returns
   `None`. `Ok(())` means "it worked, and there is no value".
2. `.ok_or("...")` converts the `Option` into a `Result`: `Some(v)` becomes `Ok(v)`, and
   `None` becomes `Err("...")`. Then `?` can return it. A `&str` can be converted into a
   `Box<dyn Error>`, so a plain message works as an error here.
3. `.into()` converts the `String` into a `PathBuf`. Rust needs to know which type you want,
   so the variable has a type annotation: `let path: PathBuf`. Without the annotation, the
   compiler reports `type annotations needed`.
4. Method calls can be chained over several lines. `cargo fmt` formats them like this.
5. `{:?}` prints a value with its `Debug` form, which you derived in Step 4.

### 15. Look at it

- [ ] Run:

  ```powershell
  cargo run -- ..\gsearch\data\db\queries\gd00000_q.json
  ```

  Expected:

  ```text
  gd00000_q: 3 nodes, 3 edges
  first node: NodeRecord { id: 8018, x: -2.620017385197598, y: -1.718125461002694 }
  ```

- [ ] Run it with a file that does not exist: `cargo run -- nope.json`. Expected:

  ```text
  Error: Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }
  ```

  The program ends with an error message, not a panic, and you can see it in the type of
  `main`.

- [ ] Now print the same node from Python:

  ```powershell
  python -c "import json; print(json.load(open('../gsearch/data/db/queries/gd00000_q.json'))['nodes'][0])"
  ```

  Expected: `{'id': 8018, 'x': -2.620017385197598, 'y': -1.7181254610026941}`

Compare `y`. Python prints `...0026941`, but Rust prints `...002694`. Both programs print
the shortest text that gives the exact same float, so they read **different floats** from the
same file. The difference is one unit in the last place (1 ULP). By default, `serde_json` uses
a fast float parser that is not always correctly rounded. Python's parser is always correctly
rounded.

This matters for the "same results as Python" goal. A 1-ULP difference in a coordinate can
move a node from inside the tolerance to outside it.

### 16. Parse floats exactly

- [ ] Run:

  ```powershell
  cargo add serde_json --features float_roundtrip
  ```

- [ ] Run Step 15's first command again. Expected:
  `first node: NodeRecord { id: 8018, x: -2.620017385197598, y: -1.7181254610026941 }`.
  It is now the same as Python.

`float_roundtrip` makes `serde_json` parse every float correctly rounded. It is a little
slower, but correct results are worth it.

### 17. Debug vs release speed

- [ ] Time a large file with the debug build:

  ```powershell
  Measure-Command { cargo run -q -- ..\gsearch\data\db\graphs\go00001.json } | Select-Object TotalSeconds
  ```

  Expected: the program prints `go00001: 452628 nodes, 5604320 edges`, and `TotalSeconds` is
  about 3 to 4.

- [ ] Time it with Python:

  ```powershell
  Measure-Command { python -c "import json; json.load(open('../gsearch/data/db/graphs/go00001.json'))" } | Select-Object TotalSeconds
  ```

  Expected: about 3 seconds as well.

- [ ] Build in release mode and time it again:

  ```powershell
  cargo build --release
  Measure-Command { .\target\release\gsearch.exe ..\gsearch\data\db\graphs\go00001.json } | Select-Object TotalSeconds
  ```

  Expected: about 0.4 to 0.6 seconds.

A debug build does no optimization and adds checks, such as integer-overflow checks. That
makes it compile fast and gives clear errors, but it can be 5 to 50 times slower than release.
**Always measure speed with `--release`.** Use debug builds for development and tests.

### 18. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
  (`--all-targets` also lints the test code.)
- [ ] Run `cargo test`. Expected: `2 passed`.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 1: read graph JSON files"
  ```

---

## Decision point: D5, graph storage

Now you have seen structs, `Vec`, and ownership. Before Slice 3, decide how to store the
graph. Read D5 in the [roadmap](README.md#design-decisions). A short summary:

- **Own compact struct (recommended).** Store `ids: Vec<NodeId>`, `coords: Vec<Point>`,
  `adjacency: Vec<Vec<usize>>`, and an `index_of: HashMap<NodeId, usize>`. You write
  Dijkstra and the component search yourself. Those are good, small exercises, and you
  see all of the code.
- **`petgraph`.** It is closer to networkx, and you write less code. But you must learn its
  generic types, such as `Graph<N, E, Undirected, u32>`.

Slice 2 does not depend on this decision, so you can continue with Slice 2 now. Tell Claude
your choice when you want Slice 3 written.

Next: [Slice 2: Geometry](slice-02-geometry.md).
