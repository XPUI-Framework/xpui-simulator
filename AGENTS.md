# `xpui-simulator`

## What this is, and what it may not become

A desktop window that runs an `xpui` app: `xpui-embedded-graphics` over
`embedded-graphics-simulator`'s display, plus an event pump, a keyboard
mapping, a mouse that becomes a finger, and a device body drawn around the
panel from a board's millimetres. The pixels are the ones a device would get,
through the same backend and the same components.

**It is a window and nothing else.** Not a rendering backend of its own, not
a device list — it opens whatever board it is handed and knows none — and not
a screen. It runs on a desktop and nowhere else, so there is no bare-metal
lint here, on purpose. It cannot be built, or documented, without SDL2
installed, which is what publishing it to docs.rs will have to solve.

## The gate

```bash
./build-and-test.sh          # everything below
./build-and-test.sh fix      # the same, formatting in place first
```

```text
format · file sizes · crates are tested · READMEs warn · prose is compiled · documented paths resolve · rustdoc links resolve · the reference mirrors rustdoc · documented commands resolve · lint · tests · doctests · README sections · AGENTS.md · published crates deny missing_docs · comment blocks · comment narration
```

There is no `all` mode; this list is the whole of it, and a last stage,
`the gate is documented`, compares it to what ran. Run it before saying a
change is done, and read the real exit code.

## What only this repository checks

No check of its own in the gate, and one absence on purpose: no bare-metal
clippy, because nothing here compiles for a device — the only other
repositories without one of their own are `xpui-cpp` and `xpui-dev`. What no
sibling has is `tests/keys.rs`, the one prose-as-test in the organisation: it
reads both key tables in `docs/running.md`, and checks every row of the first
against `button_for` and every row of the live-control table against
`control_for`.

## Style that bites here

- **No test opens a window.** Everything is driven by `(position,
  timestamp)` events and asserted on a framebuffer; the loop runs on CI with
  `--frames N` or headless. An agent never opens the simulator to report what
  it saw — that is the author's step.
- **Positions are panel pixels**, never window pixels: the window is a scaled
  view, and a threshold in window pixels moves at every zoom.
- **The touch constants are the firmware's, verbatim** (`src/touch/`). Change
  one and the model stops agreeing with the device it is a stand-in for.
- **Presses arrive raw.** What two close presses mean is the firmware's
  decision, expressed through `Keys`, never decided here.
- **The window never resizes.** `Session` opens it once at the largest board
  in the cycle and letterboxes the rest; every zoom and switch is measured
  against that one set of numbers.
- **`INK_IS_ON` is chosen in three places that must agree**: the backend's
  palette, the screenshot reader and the window's theme.
- **Every `pub` item is documented.** `#![deny(missing_docs)]` is on.
- **A file under `src/` is at most 400 lines.** `run.rs` is the one near it.

## Where the documentation lives, and what proves each piece

| Document | Proven by |
|---|---|
| [`README.md`](README.md) | its `rust` fence is a doctest (`no_run`: it opens a window), mounted by `src/lib.rs` |
| [`docs/README.md`](docs/README.md) | its paths resolve; the README-heading check exempts it, because it is the index of `docs/`, not a front page |
| [`docs/running.md`](docs/running.md) | doctests, mounted by `src/lib.rs`; both its key tables are read by `tests/keys.rs` |
| [`docs/reference.md`](docs/reference.md), [`docs/reference/`](docs/reference/) | doctests, mounted by `src/lib.rs`; every public name mirrored by `the reference mirrors rustdoc`; the picture on `panel.md` is the golden `tests/pictures.rs` draws |
| [`docs/design.md`](docs/design.md) | its paths resolve; it carries no `rust` fence |
| [`docs/contributing.md`](docs/contributing.md) | every path and command it gives resolves; the umbrella command is `xpui-dev`'s |
| `AGENTS.md` | the stage list above is compared to what the gate runs |
| every `///` and `//!` | `rustdoc links resolve`, and the two comment checks |

## Git

Never stage, never commit, never push without being asked, each time. The
index is the reviewer's queue; leave new work unstaged. No self-attribution
in a commit message. Never rewrite a commit that exists; a correction is a new
commit. The rules that apply to all ten repositories, and the five review
steps, are in [`xpui`'s `docs/orientation.md`](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/orientation.md);
how a change is built and reviewed here is in
[`docs/contributing.md`](docs/contributing.md).
