# Slice 9: Speed: how to profile

**Goal.** Learn how to find where the time and the memory go, before you change any code. This
slice is a method, not a port. The only change to your code is a build profile with debug symbols
and an optional allocation counter. It does not run the program on a large graph, and it does not
change `run_gsearch`. Slice 10 (`rayon`) comes after it, because a parallel program that is
slow for a reason that you have not measured stays slow.

**Outcome.** You can (1) build a release binary that a profiler can read; (2) time a run in a
way that you can trust; (3) take a sampling profile and read the flame graph; (4) count the
memory allocations of a run; (5) check a change by the same two numbers each time. You also have
a list of places in this code to test first.

**New ideas.** A custom Cargo profile (`inherits`, `debug`); a Cargo feature that turns code on and
off (`#[cfg(feature = "...")]`); an optional dependency; the global allocator; a guard value
whose `Drop` does the work (`let _profiler`, and why it is not `let _`); sampling profilers;
what the numbers of `dhat` mean.

**Starting point.** Slice 8 is done: `cargo test` reports `0 failed`, and the CLI runs on
`gr00049` with `-e 9`.

**What I ran and what I did not.** I ran every command in Steps 1, 2 and 4 on `gr00049`, and the
numbers in this slice are from those runs on my machine. Yours will differ, so compare **your own**
runs with each other, and use mine only to check that the order of magnitude is right. I did
**not** run `samply` (Step 3): it is not installed on this machine. The commands for it are from
its documentation, and the first run may need a small change.

---

## The rules

1. **Measure a release build.** A debug build is often 10 to 50 times slower, and the slow parts
   are not the same (bounds checks and missing inlining dominate it). Slice 0, Step 7 explains
   the two builds.
2. **Measure the same work each time.** The same files, the same arguments, the same machine
   state (plugged in, no other heavy program). Compare numbers only from the same setup.
3. **Run it more than once and take the median.** One run is not a measurement. The first run
   also warms the file cache.
4. **Look before you change.** A profile says where the time is. A guess is often wrong, and
   the part that looks slow in the code may take 1% of the time.
5. **One change, one measurement, one commit.** If two changes go in together, you cannot say
   which one helped.
6. **A faster program must give the same answer.** After each change, check the output (for this
   program: the number of mappings, and when the placeholders are gone, the lowest score).

---

## The workload

Use `gr00049` (1128 nodes) with `-e 9`:

```powershell
cargo run --profile profiling -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json -e 9
```

It has 8 498 802 mappings and takes about 4.7 s in a release build with the placeholders. This is
long enough for a profiler (it takes about 1000 samples each second by default, so it gets thousands), and
short enough to repeat. Do **not** use `-e 10` (281 million mappings, about 168 s) or a large
graph.

**The placeholders change what you see.** With the placeholder `find_paths`, the profile shows
`get_candidate_matches`, `make_crop` and the allocation work of `align_and_score`. It does not show
Dijkstra. After you write `find_paths`, it will probably be the top item, so **profile again**, and
do not trust a list that was made with the placeholder.

---

## Steps

### 1. A profile that has symbols

A release build has no names for its functions in the file, to keep the file small. A profiler
shows only addresses then. You want the full optimisation of release, **and** the names. Cargo can
make a second profile that inherits from `release` and changes one setting.

- [ ] Add to the end of `Cargo.toml`:

  ```toml
  [profile.profiling]
  inherits = "release"
  debug = true
  ```

  1. `[profile.NAME]` makes a build profile. `inherits = "release"` takes every setting of
     `release` (optimisation level 3 and so on) and `debug = true` adds the debug information.
     The code that is generated is the same, and the file is bigger.
  2. Why not `debug = true` in `[profile.release]`? Then every release build is bigger. A
     separate profile keeps `cargo build --release` as it was.
  3. On Windows the debug information is in a separate file, `gsearch.pdb`, next to the program.

- [ ] Run `cargo build --profile profiling`. Expected: `Finished 'profiling' profile
  [optimized + debuginfo]`. The program is `target\profiling\gsearch.exe`, and `gsearch.pdb` is in the
  same folder. (A profile that is not `dev` or `release` has its own folder under `target`.)

### 2. A timing that you can trust

The program prints its own time, but it measures only the search. To time the whole process,
use `Measure-Command` of PowerShell. It is built in, and no tool needs to be installed.

- [ ] Run:

  ```powershell
  $q = "data\roads\queries\gr00049_q.json"; $g = "data\roads\graphs\gr00049.json"
  1..5 | ForEach-Object { (Measure-Command { .\target\profiling\gsearch.exe $q $g -e 9 | Out-Null }).TotalSeconds }
  ```

  Expected: five numbers that are close to each other. My three runs were `4.67`, `4.57` and
  `4.76` seconds. (The `INFO` line of the logger is on standard error, so it still shows on the
  screen. `Out-Null` hides only standard output.) Write down the **median** of your five numbers.
  This is your baseline.

  If the numbers differ by more than about 10%, the machine is busy. Close other programs, plug
  in the power cable, and run again. If you have the tool `hyperfine` (`winget install
  sharkdp.hyperfine`), it does the repeating and the statistics, and prints a mean with the
  spread. I did not run it.

### 3. Where does the time go? A sampling profiler

A **sampling profiler** stops the program many times each second, writes down which function is
running (and the functions that called it), and counts. A function that is in 30% of the samples
uses about 30% of the time. It slows the program very little, and it needs no change to the code.

The tool that I recommend is `samply`. It works on Windows, and it shows the result in a browser.

- [ ] Install it once: `cargo install samply`.
- [ ] Run it on the profiling binary:

  ```powershell
  samply record .\target\profiling\gsearch.exe data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json -e 9
  ```

  When the program ends, `samply` opens the result in your browser (the Firefox Profiler, from
  a local web server). If it does not, read the text that it prints: it says how to open it.
  Use `Ctrl+C` to stop `samply` when you are done.

  If `samply` does not work on your machine, two other ways: Visual Studio's *Debug, Performance
  Profiler, CPU Usage* can start `target\profiling\gsearch.exe`, and `cargo flamegraph` (which needs
  an administrator terminal on Windows). Both read the `.pdb` file.

- [ ] Look at the result in this order:

  1. **Call Tree**, with **Invert call stack** ticked. This lists the functions that have the most
     **self time**: the time spent in the function itself, not in what it calls. The top three or
     four lines are where to look.
  2. **Flame Graph**. Each box is a function, its width is its share of the time, and a box sits on
     top of the function that called it. A wide box with narrow boxes on top means "the time is in
     this function". A wide box that is covered by other wide boxes means "the time is in the
     callees".
  3. Click a function: the profiler shows the source line, if it can find the file. This is
     the reason for `debug = true`.

  Things that confuse a first reading:

  - **Inlining.** The compiler copies small functions into their callers. A function that you
    wrote may not be in the list, and its time is added to its caller. The debug information
    still lets the profiler show the line.
  - **`alloc`, `malloc`, `free`, `memcpy` near the top** mean "the program makes and copies
    memory a lot". The profile shows that it happens. Step 4 shows where.
  - **Self time and total time are different.** `run_gsearch` has a high total time, because
    it calls everything, and a low self time.
  - The first and the last second of a run may show loading and writing, and not the search.
    Only the middle is the work that you want to speed up.

- [ ] Write down, for yourself, the top five functions by self time and their share.

### 4. Where does the memory go? Counting allocations

A call that gets memory from the heap (a `Vec` that grows, a `String`, a `HashMap`) takes time.
A loop that does it millions of times shows as `malloc`/`free` in the profile of Step 3.
`dhat` is a Rust crate that counts every allocation of a run and says **which line** made it.

- [ ] Add it as an **optional** dependency:

  ```powershell
  cargo add dhat --optional
  ```

  Cargo adds the line `dhat = { version = "...", optional = true }` and a feature with the same
  name in a new `[features]` section: `dhat = ["dep:dhat"]`. An optional dependency is not built
  unless the feature is on. A normal build does not have `dhat`, and a normal program is not
  slower.

- [ ] In `src/main.rs`, add this before the doc comment of `Args`:

  ```rust
  #[cfg(feature = "dhat")]
  #[global_allocator]
  static ALLOC: dhat::Alloc = dhat::Alloc;
  ```

  and this as the first lines of `main`:

  ```rust
      #[cfg(feature = "dhat")]
      let _profiler = dhat::Profiler::new_heap();
  ```

  1. `#[cfg(feature = "dhat")]` removes the line that follows from the program, unless the
     feature `dhat` is on. This is **conditional compilation**. In Python, you would write an
     `if` that runs on each start. Here the other code does not exist in the normal program.
  2. `#[global_allocator]` tells Rust which code gives out heap memory to the whole program.
     `dhat::Alloc` gives memory like the normal allocator, and counts every call.
  3. `dhat::Profiler::new_heap()` makes a **guard** value. When the guard is dropped, at the end
     of `main`, it writes the file `dhat-heap.json` and prints a summary. Dropping a value at the
     end of its scope is how Rust frees things (Slice 3, Step 9).
  4. The name must be `_profiler`, with a letter after the `_`. `let _ = ...` drops the value
     **at once**, and the profile would be empty. A name that starts with `_` is "not used, but
     keep it until the end of the scope" (Slice 6, Step 5 explains the `_` prefix).

- [ ] Run it on a **smaller** workload (`-e 8`): `dhat` makes the program about ten times slower.

  ```powershell
  cargo run --profile profiling --features dhat -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json -e 8
  ```

  Expected (my run; the numbers are the same on every run, because they count calls):

  ```text
  dhat: Total:     27,779,266 bytes in 227,743 blocks
  dhat: At t-gmax: 951,546 bytes in 6,037 blocks
  dhat: At t-end:  1,445 bytes in 9 blocks
  dhat: The data has been saved to dhat-heap.json, and is viewable with dhat/dh_view.html
  ```

  1. **Total** is everything that the run asked for: 227 743 allocations ("blocks") and 27.8 MB.
     The number of blocks is the one that costs time.
  2. **At t-gmax** is the moment of the highest memory use. It is about 1 MB here, because only
     one anchor's mappings are in memory at a time.
  3. **At t-end** is what is left at the end. It should be small. A large number means a leak,
     or a value that is kept for the whole run.
  4. The same run with `-e 7` (12 234 mappings) has 157 426 blocks, and with `-e 6` (408
     mappings) 68 755. So each extra mapping costs about **7 or 8 blocks**, roughly. With millions
     of mappings, that is tens of millions of calls to the allocator. This is your first number to
     bring down, and the next step shows where they come from.

- [ ] Open `dhat-heap.json` in the viewer. The viewer is a web page that reads a local file: the
  DHAT viewer (`dh_view.html`, from the Valgrind project; the `dhat` crate documentation has the
  link). I did not open it. Use the **Total (blocks)** sort, and open the stack of the top
  entries. Each entry shows the line that allocated, with the stack of callers. The top entries
  should name functions of your code (for example `filter_matches`, `extend_poses`/`generate_candidates` (the names in your file),
  `assemble`, `Graph::edges`, `Graph::subgraph`). Write down the top five.

- [ ] Add these two lines to `.gitignore`, so that the output is not committed:

  ```text
  dhat-heap.json
  profile.json*
  ```

### 5. What to look at first

These are guesses, each with a way to check it. They come from reading the code. A guess is
**not** a result. Do the one that the profile of Steps 3 and 4 points at.

| Place | Why it may be slow | How to check | A way to fix it |
|---|---|---|---|
| `align_and_score` (Slice 7) | Several `Vec`s and two `BTreeSet`s for each mapping (`targets`, `moved`, `assemble`, the ids). By my count, with the placeholders it makes about 6 blocks for each mapping (`targets`, `moved`, the node set, the lists of ids), and `generate_candidates` makes 1 more (the clone of the mapping). That is close to the 7 or 8 that Step 4 measures. It is a count from reading, so check it in `dhat`. | The top entries in `dhat`; `alloc` near the top in `samply` | Keep the buffers and reuse them; do the cheap work first and build the `Alignment` only if its score can enter `TopK` (the score is known before the ids are made) |
| `Graph::edges` (Slice 7, Step 1) | A new `Vec` for each call, and `find_paths` calls it for each mapping. The placeholder does not call it, so this shows only after you write `find_paths` | `dhat`: a `Graph::edges` entry with a block count equal to the mapping count | Return an iterator, or compute the edges of `Q` once and pass them in |
| `filter_matches` (Slice 5) | Three new tables for each alignment node, and one `Vec` for each row | `dhat` | Filter in place, or test the pair while the search goes down (Slice 5 said that Slice 9 would come back to this) |
| `generate_candidates` (Slice 5) | All the mappings of an anchor are stored before the first is used | `At t-gmax` grows with the largest anchor | Give each mapping to the next step when it is found (a closure or an iterator), and stop early on a bad one (Slice 5 said that Slice 9 would come back to this too) |
| `SpatialIndex::make_crop` (Slice 4) | It builds a new `Graph` (with a `HashMap`) for each of the 1128 anchors | `samply`: `subgraph` and `cc_containing` high in the tree | Decision D6 named the other way: a breadth-first search that steps only onto nodes inside the radius, with no k-d tree. Try it and compare |
| `HashMap` | The default hash function is built to resist attacks, and it is slow for plain integer keys | `samply`: `SipHasher` or `hash` high in the tree | A faster hasher, for example the crate `rustc-hash`. Do it only if the profile shows hashing |
| `find_paths` (yours) | One Dijkstra for each query edge for each mapping, with a distance `Vec` of the size of the crop each time | `samply` after you write it | Reject a mapping early when an end node is too far from its line; reuse the buffers |
| The number of mappings | 281 million at `-e 10`. This is a problem of the **algorithm**. | The table in Slice 8 | A ranking module for the anchors; checks inside the backtracking (for example the length of each query edge) so that a bad branch stops early |

**The last row is the largest.** A change that cuts the number of mappings by a factor of 100
is worth more than all the other rows together. The tolerance table of Slice 8 shows it: `-e 8`
and `-e 9` differ by 400 times in mappings, and by 200 times in time. A faster loop saves a
fraction of the time, and a smaller search saves the rest.

### 6. Change one thing, measure again

For each change:

- [ ] Run the tests: `cargo test`. A faster program that fails a test is not faster.
- [ ] Run the workload of Step 2 (five runs, median). Compare with the baseline.
- [ ] Check the output: the line `1128 anchors, 8498802 mappings, 1 alignments kept` must be the
  same, with the same `lowest score`. Write both down before your first change. A change that
  alters the number of mappings or the lowest score has changed the answer, and it is a bug,
  not a speed-up.
- [ ] Write the result in your commit message, for example `perf: reuse the buffers in align_and_score (4.6 s -> 3.9 s)`.
  If the change did not help, say that, and undo it: `git restore src`. A failed experiment that is
  written down is useful.

### 7. Settings of the compiler to try (last, and one at a time)

These change how `rustc` builds the whole program, not your code. The gain is often 5% to 20%, and
it can be zero. Measure each one by the method of Step 2.

- [ ] In `[profile.release]` (and the same in `[profile.profiling]`, so that the profile
  matches), try, one at a time: `lto = "fat"` (the compiler optimises across all crates, and the
  build is slower), `codegen-units = 1` (the compiler splits the work in fewer parts, and the
  build is slower, and the result is often faster), and `panic = "abort"` (a panic stops the
  program at once, with no cleanup, which makes the code a little smaller).
- [ ] Try `RUSTFLAGS="-C target-cpu=native"` (PowerShell: `$env:RUSTFLAGS = "-C target-cpu=native"`,
  then build). The program uses the newest instructions of **your** processor, and may not run on
  another one. Do not use it for a program that you give to other people.

### 8. Later: the large graphs (do not do this now)

You said no large graph for now. This is the list for when you do:

1. Write `fit` and `find_paths`, so the scores are real, and check the six small graphs (Slice 8,
   Step 9).
2. Profile again, with `-e 9` on `gr00049`, because Dijkstra will have changed the picture.
3. Add a limit on the number of anchors (a `--max-anchors N` option in `Args` and a `.take(n)` on
   the anchor list), or the ranking module that you plan. A run on a graph with 10^5 nodes tries
   every node as an anchor today.
4. Start with the smallest large graph in `..\gsearch\data\db\graphs\manifest.jsonl`, with
   `-e 2` or `-e 3`, and watch Task Manager. Stop with `Ctrl+C` if the memory climbs.
5. Only then Slice 10: `rayon` over the anchors. The loop of `run_gsearch` has an independent
   body for each anchor (it only changes `TopK` and the two counters), so it fits. `TopK` and
   `n_mappings` then become per-thread values that you merge at the end.

### 9. Commit

- [ ] Run `cargo fmt; cargo clippy --all-targets; cargo test`.
- [ ] Commit:

  ```powershell
  git add Cargo.toml Cargo.lock src .gitignore guide
  git commit -m "chore: profiling build profile and optional dhat"
  ```

---

## What comes next

Slice 10 runs the anchors in parallel with `rayon`. Slice 11 (optional) calls the search from
Python with `PyO3`. Neither is written yet.
