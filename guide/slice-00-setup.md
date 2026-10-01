# Slice 0: Toolchain and first program

**Goal.** Get a current Rust toolchain and editor support, and make the `gsearch` Cargo
project in this folder. You also read your first Rust compiler error, because reading these
errors is most of the work in your first weeks of Rust.

**Outcome.** `cargo run` prints `Hello, gsearch!`, `cargo clippy` reports no problems, and the
project has its first git commit.

**New ideas.** `rustup`, `rustc` and `cargo`; `Cargo.toml`; `fn main`; macros (`println!`);
debug builds; `let` vs `let mut`; compiler error codes; `cargo fmt` and `cargo clippy`.

**Starting point.** Windows, PowerShell, VS Code. All commands run in
`C:\Users\jesse_8eqas6o\Documents\code\gsearch_rs` unless the step says otherwise.

---

## Steps

### 1. Check for Rust

- [ ] Run:

  ```powershell
  rustup --version
  rustc --version
  ```

  Expected: two version lines, for example `rustc 1.99.0 (b940084d7 2026-09-28)`. This machine
  already had Rust installed (it showed 1.81 at first), so you probably see a version.

  If PowerShell says `rustup` is not recognized, install Rust: download `rustup-init.exe` from
  <https://rustup.rs>, run it, and choose the default install. On Windows, Rust uses the
  Microsoft C++ linker. If the installer offers to install the Visual Studio Build Tools,
  accept. Then close and reopen the terminal, and run this step again.

There are three tools. `rustup` installs and updates Rust versions, like `uv python install`.
`rustc` is the compiler. You almost never call it directly. `cargo` is the build tool and
package manager, like `uv`. It builds, runs, tests, and adds dependencies.

### 2. Update to the current stable version

- [ ] Run:

  ```powershell
  rustup update stable
  rustc --version
  ```

  Expected: the last line shows version 1.85 or newer.

The guide uses the **2024 edition** of Rust, which needs 1.85 or newer. An edition is a set of
language rules that a project selects in `Cargo.toml`. Python has no direct equivalent.
Editions let Rust make small breaking changes without breaking old projects, because each
project keeps the edition it selected.

`~/.cargo/bin` may also contain `rls.exe`. That is an old editor tool. You can ignore it.

### 3. Install the editor extension

- [ ] Run:

  ```powershell
  code --install-extension rust-lang.rust-analyzer
  ```

  Expected: `Extension 'rust-lang.rust-analyzer' ... was successfully installed.`

- [ ] In VS Code, look at the Extensions panel. If an extension named just **Rust**
  (`rust-lang.rust`) is installed, uninstall it. It is the old extension, and it conflicts
  with rust-analyzer.

rust-analyzer does for Rust what Pylance does for Python. It also shows the inferred type of
each variable in grey next to the code. When you are learning Rust, these type hints are very
useful.

### 4. Make the Cargo project

- [ ] Run:

  ```powershell
  cargo init --name gsearch
  ```

  Expected: `Creating binary (application) package`.

`cargo init` uses the current folder (`cargo new <name>` makes a new folder). Your `.claude`
and `.llm` folders stay where they are. Because this folder was not a git repository, Cargo
also ran `git init` and wrote a `.gitignore` that contains `/target`.

### 5. Look at what Cargo made

- [ ] Open `Cargo.toml`. It contains:

  ```toml
  [package]
  name = "gsearch"
  version = "0.1.0"
  edition = "2024"

  [dependencies]
  ```

This is your `pyproject.toml`. `[dependencies]` is the same as `dependencies = [...]`. There is
no separate build backend to configure, because Cargo is the only build system.

### 6. Read `src/main.rs`

- [ ] Open `src/main.rs`:

  ```rust
  fn main() {
      println!("Hello, world!");
  }
  ```

`fn main` is where the program starts, like `if __name__ == "__main__":`. A binary must have
it. The `!` in `println!` means it is a **macro**, not a function. A macro writes code at
compile time. `println!` uses a macro because it checks the format string against its
arguments when it compiles, so `println!("{} {}", x)` is a compile error, not a runtime error.

### 7. Build and run

- [ ] Run:

  ```powershell
  cargo run
  ```

  Expected:

  ```text
     Compiling gsearch v0.1.0 (C:\Users\jesse_8eqas6o\Documents\code\gsearch_rs)
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.6s
       Running `target\debug\gsearch.exe`
  Hello, world!
  ```

`cargo run` compiles and then runs. The program is now a real `.exe` in `target\debug\`. You
can run that file directly, without Cargo or Rust installed. `dev` means a **debug build**. It
compiles fast but runs slowly. Slice 1 shows how large the difference is.

### 8. Your first compiler error

- [ ] Replace the contents of `src/main.rs` with:

  ```rust
  fn main() {
      let name = "gsearch";
      println!("Hello, {name}!");
  }
  ```

- [ ] Run `cargo run`. Expected: `Hello, gsearch!`

`let` makes a variable. `{name}` in the format string puts a variable in directly, like an
f-string.

- [ ] Now add one line at the end of `main`, after the `println!`:

  ```rust
      name = "rust";
  ```

- [ ] Run `cargo run`. Expected: the build fails.

  ```text
  error[E0384]: cannot assign twice to immutable variable `name`
   --> src\main.rs:4:5
    |
  2 |     let name = "gsearch";
    |         ---- first assignment to `name`
  3 |     println!("Hello, {name}!");
  4 |     name = "rust";
    |     ^^^^^^^^^^^^^ cannot assign twice to immutable variable
    |
  help: consider making this binding mutable
    |
  2 |     let mut name = "gsearch";
    |         +++
  ```

In Rust, a variable cannot change unless you mark it `mut`. In Python everything can change,
and you use `Final` hints to ask a checker to prevent it. Rust has the opposite default. Then,
when you see `let mut` in code, you know that the value changes later.

Read the error from top to bottom. The first line has a code (`E0384`) and a summary. The
picture points to the lines involved. The `help:` section often contains the fix. For a
longer explanation, run `rustc --explain E0384`.

- [ ] Change `let name` to `let mut name`, and run `cargo run` again.

  Expected: the program runs, but first prints
  `warning: value assigned to `name` is never read`.

A warning does not stop the build. The compiler found that `"rust"` is assigned but never
used. Rust programmers usually fix every warning.

- [ ] Remove the `name = "rust";` line and change `let mut` back to `let`. Run `cargo run`
  again. Expected: `Hello, gsearch!` with no warnings.

### 9. Look at it

- [ ] Run the program without Cargo:

  ```powershell
  .\target\debug\gsearch.exe
  ```

  Expected: `Hello, gsearch!`

### 10. Format and lint

- [ ] Run:

  ```powershell
  cargo fmt
  cargo clippy
  ```

  Expected: `cargo fmt` prints nothing. `cargo clippy` ends with `Finished` and shows no
  warnings.

`cargo fmt` is the one standard formatter, like `ruff format`. Nobody configures it, so all
Rust code looks the same. `cargo clippy` is the linter, like `ruff check`. It often suggests a
more idiomatic way to write something, which helps when you are learning. Run both before every
commit.

### 11. Commit

- [ ] Run:

  ```powershell
  git add -A
  git commit -m "Slice 0: cargo project skeleton"
  ```

  Expected: `[main (root-commit) ...] Slice 0: cargo project skeleton`, plus a list of files
  (your branch name can be `master` instead of `main`).

---

Next: [Slice 1: Read a graph file](slice-01-read-json.md).
