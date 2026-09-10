[![CI](https://github.com/XPUI-Framework/xpui-simulator/actions/workflows/ci.yml/badge.svg)](https://github.com/XPUI-Framework/xpui-simulator/actions/workflows/ci.yml) [![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# `xpui-simulator`

> [!WARNING]
> Under heavy development. Not production-ready. The API can break without
> notice. Use at your own risk.

Runs an [`xpui`](https://github.com/XPUI-Framework/xpui-framework) app in a
desktop window, so screens can be developed without hardware. Not a separate
rendering backend: it is
[`xpui-embedded-graphics`](https://github.com/XPUI-Framework/xpui-backends/tree/main/embedded_graphics)
over `embedded-graphics-simulator`'s display, plus a window, an event pump and
a keyboard mapping. **The pixels are the ones a device would get** — same
backend, same components, same measurements.

## Using it

```toml
[dependencies]
xpui-simulator = { git = "https://github.com/XPUI-Framework/xpui-simulator", branch = "main" }
```

```rust,no_run
# use xpui::{Screen, Text, View};
# struct MainMenu;
# impl MainMenu { fn new() -> Self { MainMenu } }
# impl Screen for MainMenu {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new("Main menu") }
#     fn update(&mut self, _message: ()) {}
# }
use xpui_simulator::{Board, Panel, Simulator};

fn main() {
    // Your panel. `xpui-boards-pimoroni`, `-xteink` and `-seeed` carry
    // ready-made ones.
    let mine = Board::custom("my reader", 480, 800, false);
    Simulator::new(Panel::of(mine))
        .title("my reader")
        .run(MainMenu::new());
}
```

`cargo run -p xpui-gallery` — the seven-board gallery this window was built
around — lives in [`xpui-gallery`](https://github.com/XPUI-Framework/xpui-gallery),
not here; this crate knows no device and no screen, and opens whatever board
and screen it is handed. Nothing is on crates.io yet, which is why the dependency is
a `git` URL.

## Requirements

SDL2, which `embedded-graphics-simulator` needs.

```bash
brew install sdl2           # macOS
apt install libsdl2-dev     # Debian/Ubuntu
```

## Checking it

```bash
./build-and-test.sh
```

The checks are in [`xtask/`](xtask/) — this repository's own list, in Rust,
holding nothing it does not run. There is no bare-metal lint here and there is
no C++ stage: this crate opens a window, and runs on a desktop and nowhere
else. `./build-and-test.sh fix` formats in place first. How a change is
reviewed is in [docs/contributing.md](docs/contributing.md).

## Where next

| | |
|---|---|
| [docs/running.md](docs/running.md) | the operator's guide, in the sections below |
| [Running the gallery](docs/running.md#running-the-gallery) | `cargo run -p xpui-gallery -- --board x3`, and the seven slugs it takes |
| [Boards](docs/running.md#boards) | `Panel::of(board)`, and `Simulator::boards(&[..])` for the cycle `B` walks |
| [Keys and mouse](docs/running.md#keys-and-mouse) | what every key sends, and how a click becomes a tap |
| [Changing it while it runs](docs/running.md#changing-it-while-it-runs) | board, zoom, body, screenshot — without restarting |
| [The device around the panel](docs/running.md#the-device-around-the-panel) | the bezel drawn from millimetres, with clickable keys |
| [`--frames N`](docs/running.md#--frames-n) | why the loop is testable, and how CI runs it |
| [The mouse as a finger](docs/running.md#the-mouse-as-a-finger) | a drag is a swipe, the wheel is a scroll, and where the edge gestures live |
| [Headless](docs/running.md#headless) | screenshots with no window, and no window at all |
| [When it does not run](docs/running.md#when-it-does-not-run) | the failures people actually hit |
| [docs/design.md](docs/design.md) | why the window redraws when nothing changed, and the other arguments behind choices the code states in one sentence |
| [docs/contributing.md](docs/contributing.md) | SDL2 first, then building it, the gate, the five review steps, and how a commit is written |

## Where it sits

Every arrow is a dependency in a `Cargo.toml`, and they all point inward
toward `xpui`, which depends on nothing at all. That is the rule the
organisation is arranged around: a backend can be written without the framework
knowing it exists, and a firmware reaches whatever it needs directly rather
than through whoever happens to sit above it.

```mermaid
flowchart BT
  xpui["xpui<br/>the framework"]
  chrome["xpui-chrome<br/>components"]
  boards["xpui-boards<br/>seven devices"]
  backends["xpui-backends<br/>two backends"]
  simulator["xpui-simulator<br/>a window"]
  gallery["xpui-gallery<br/>the app"]
  rp2040["xpui-rp2040<br/>firmware"]
  esp32["xpui-esp32<br/>firmware"]
  cpp["xpui-cpp<br/>a C++ host"]
  dev["xpui-dev<br/>the umbrella"]
  chrome --> xpui
  boards --> xpui
  backends --> xpui
  backends --> chrome
  simulator --> xpui
  simulator --> chrome
  simulator --> boards
  simulator --> backends
  gallery --> xpui
  gallery --> chrome
  gallery --> boards
  gallery --> backends
  gallery --> simulator
  rp2040 --> xpui
  rp2040 --> boards
  rp2040 --> backends
  rp2040 --> gallery
  esp32 --> xpui
  esp32 --> boards
  esp32 --> backends
  esp32 --> gallery
  cpp --> xpui
  cpp --> backends
  dev --> xpui
  dev --> chrome
  dev --> boards
  dev --> backends
  dev --> simulator
  dev --> gallery
  style simulator stroke-width:3px
```

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Thiago Holanda.
