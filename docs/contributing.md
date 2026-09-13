# Contributing to `xpui-simulator`

## Building it

SDL2 first: `embedded-graphics-simulator` links it, and without it nothing
here compiles; [`README.md`](../README.md) gives the command for macOS and
for Debian and Ubuntu. Then `rust-toolchain.toml` pins the rest, and every
other dependency is a sibling repository fetched on `main`, apart from
`embedded-graphics` and its simulator crate from crates.io.

```bash
cargo test                           # the suite, on a laptop; no window opens
./build-and-test.sh                  # everything CI checks
```

## The gate

A change is not finished until `./build-and-test.sh` passes. It is the same
command CI runs, so a green run locally means what a green tick means there.
The checks are listed in [`AGENTS.md`](../AGENTS.md) and implemented in
[`xtask/`](../xtask/); `./build-and-test.sh fix` formats in place first.

What bites here — no test opens a window, both key tables in
[`docs/running.md`](running.md) are read by a test, the touch constants are
the firmware's verbatim — is in `AGENTS.md`'s `## Style that bites here`,
once.

## The review

Five steps, in order, none skipped:

1. The gate passes, with the real exit code read.
2. The [code-reviewer](../.claude/agents/code-reviewer.md) agent reviews the
   change — every finding resolved, not noted.
3. The [docs-reviewer](../.claude/agents/docs-reviewer.md) agent reviews the
   prose, last: it runs every command a document gives and resolves every
   snippet against the API.
4. The author opens the window and looks. That is their step: an agent never
   opens the simulator to report what it saw.
5. They say commit.

## Commits

The subject says what was done — imperative, under fifty characters, one
concern. The body says what changed and why, in under about ten lines,
carrying the fact that is not in the diff. Nothing about how the bug was
found. No self-attribution.

## Working across the repositories

The gallery and the tutorial in `xpui-gallery` depend on this crate through a
`git` dependency on `main`. Before pushing a change to what a key does or
what the window shows, run the umbrella:

```bash
for d in ../xpui*/; do git -C "$d" fetch --quiet --all; done
cd ../xpui-dev && ./build-and-test.sh cross
```

It builds every crate from the sibling checkouts on disk and says which one
broke. `cross` is that repository's gate, not this one's — run it from there,
not here. The fetch first, because its link check resolves every
`github.com/XPUI-Framework/…` URL against each sibling's `origin/main`, and a
stale remote is a stale answer. `xpui`'s [`docs/orientation.md`](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/orientation.md)
describes the layout it expects.
