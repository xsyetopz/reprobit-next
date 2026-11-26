# Contributing

ReproBit is one Rust crate, `crates/reprobit`, that builds the `rbit` binary. `just` runs every
task; `just` alone lists them.

```sh
just setup              # pinned tools and git hooks
just rust::test         # the behaviour tests
just lint               # clippy, rustfmt, docs, markdown
just verify             # everything CI runs
```

## Tests

Tests drive the `rbit` binary as a user or agent would, in
[`crates/reprobit/tests/rbit.rs`](crates/reprobit/tests/rbit.rs): command lines in, exit codes
and `--json` out. Each test gets a private tool home and a catalog whose compiler is a local
shell script, so no test touches the network. Test behaviour a user can observe, not how a
module computes it.

## Catalog entries

Add a tool to [`catalog/tools.toml`](catalog/tools.toml) with a digest for every download: a
`sha256` for a URL, a full commit for git, or a pinned image for OCI. Install it with
`rbit tool install <id>` on each host the entry covers, and run one of its commands. Vendor
compilers are fetched from the archives that already host them; never add binaries to this
repository.

## Schemas

[`schemas/`](schemas) holds the output of `rbit schema project` and `rbit schema catalog`. A
test fails when they drift; regenerate them after changing the project or catalog types:

```sh
cargo run -q -- schema project > schemas/project.schema.json
cargo run -q -- schema catalog > schemas/catalog.schema.json
```

Schemas carry no version. Add fields with defaults; give a field whose meaning changes a new
name.

## Commits

Commit messages follow Conventional Commits; the commit-msg hook checks them.
