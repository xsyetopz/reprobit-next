# Agents

<!-- bedrock -->
- Run each task through `just`. `just` lists the recipes. Check each change with the narrowest one, such as `just rust::test` or `just lint`, and run `just verify` once before you finish a change that alters behavior.
- Fix the cause of each lint, type, or guard error in the code. Keep the shared configs as they are; the bedrock that wrote them owns them.
- Put project recipes in `project.just`. The `justfile` and `.just/` are shared and get replaced on the next adopt. A recipe there named `project-<task>`, such as `project-verify` or `project-test`, runs after the languages' `<task>`, so a project extends `just verify` there. `project-pre-typecheck` runs before the TypeScript type check, for a build the types import.
- Write commit messages as Conventional Commits; the commit-msg hook checks them.
- When a recipe reports a missing tool, run `just setup`, which installs the pinned ones. Install any it can't, such as `just`, Bun, rustup, or Xcode, with the system's package manager, then rerun the recipe. Tell the user what you installed, and keep the check instead of skipping it.
<!-- /bedrock -->

<!-- rust-bedrock -->
- Keep Rust unit tests out of the file they test: declare `#[cfg(test)] mod tests;` with no `#[path]`, and put the body in `foo/tests.rs` beside `foo.rs`, or in `tests.rs` beside `lib.rs`, `main.rs`, or `mod.rs`. `bedrock guard rust` rejects a `#[cfg(test)] mod name { … }` with its body in the file, and `just lint-fix` moves it to `name.rs` in that same place, such as `foo/test_support.rs`. A file that only compiles under test, such as `tests.rs` and the modules below it, may hold `#[cfg(test)]` modules of its own.
- Silence a lint only with `#[expect(lint, reason = "…")]`. Clippy rejects `#[allow]`, and an `#[expect]` that no longer fires.
- Start each integration test in `tests/` with `#![cfg(test)]`, and give its helpers `pub(crate)`.
- Name module files `foo.rs` beside `foo/`, not `foo/mod.rs`.
- Document each item the crate exports, with a `# Panics` section where it can panic and `# Errors` where it returns `Result`. `pub(crate)` and private items need no docs; `just lint` runs `cargo doc`, which rejects public docs that link to private items.
- Cargo compiles through kache, the build cache `just setup` installs and turns on. When a build is slower than it should be, `kache stats --last-build` and `kache explain` show which crates missed the cache and why.
<!-- /rust-bedrock -->
