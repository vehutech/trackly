# Contributing to Trackly

Thanks for your interest in improving Trackly! This project is meant to be hacked on —
contributions of all sizes are welcome, from typo fixes to whole new surfaces.

## Ground rules

- Be kind and constructive. See the [Code of Conduct](CODE_OF_CONDUCT.md).
- Small, focused PRs are easier to review and land faster than big ones.
- Discuss large or breaking changes in an issue first, so we agree on direction
  before you invest the effort.

## Getting set up

You'll need the [Rust toolchain](https://rustup.rs) (stable, 1.90+).

```sh
git clone https://github.com/vehutech/trackly
cd trackly

cargo build            # build the workspace (trackly-core + trackly-cli)
cargo test             # run the tests
cargo run -p trackly-cli -- --help   # run the CLI locally
```

## Project layout

```
crates/
  trackly-core/   # plan model, store, scoring, markdown parsing, HTML report — UI-agnostic
  trackly-cli/    # the `trackly` command (thin shell over the core)
  trackly-mcp/    # the `trackly-mcp` MCP server (thin shell over the core)
src-tauri/        # the desktop app's Rust backend (Tauri; depends on trackly-core)
src/              # the desktop app's React frontend
```

**Rule of thumb:** logic that more than one front door would need belongs in
`trackly-core`. Keep `trackly-cli` and `trackly-mcp` thin — the CLI is argument parsing
and terminal presentation; the MCP server is protocol plumbing and tool schemas.

## Before you open a PR

Please make sure these pass locally:

```sh
cargo fmt --all              # format
cargo clippy --all-targets   # lint (no warnings, please)
cargo test                   # tests
```

Add tests for new behavior in `trackly-core` where it's practical — the parser and
scoring are the easiest and most valuable to cover.

## Commit & PR style

- Write clear commit messages: a short imperative summary line, details below if needed.
- Reference any issue the PR closes (`Closes #123`).
- Describe *what* changed and *why* in the PR body. Screenshots help for report changes.

## Good first contributions

- New checkbox conventions or plan-doc formats in `trackly-core/src/parse.rs`.
- Additional `SEED_DOCS` filenames Trackly recognizes.
- Report styling / print-layout improvements in `trackly-core/src/report.rs`.
- More tests around scoring and parsing edge cases.

See the roadmap in the [README](README.md#roadmap) for larger tracks (MCP server, git
evidence hook, desktop app).

## Reporting bugs & requesting features

Open an issue using the templates. For bugs, include your OS, the command you ran, and
what you expected vs. what happened.

Happy hacking!
