# `xpui-simulator`

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

Runs an [`xpui`](https://github.com/XPUI-Framework/xpui-framework/tree/main/crates/xpui) app in a desktop window, so screens can be
developed without hardware.

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
    // ready-made ones; this crate knows no device and opens whatever it is
    // handed.
    let mine = Board::custom("my reader", 480, 800, false);
    Simulator::new(Panel::of(mine))
        .title("my reader")
        .run(MainMenu::new());
}
```

Not a separate rendering backend: it is
[`xpui-embedded-graphics`](https://github.com/XPUI-Framework/xpui-backends/tree/main/embedded_graphics) over
`embedded-graphics-simulator`'s display, plus a window, an event pump and a
keyboard mapping. **The pixels are the ones a device would get** — same
backend, same components, same measurements.

## What it does, and where that is written down

[`docs/running.md`](docs/running.md) is the guide.

| | |
|---|---|
| [Running the gallery](docs/running.md#running-the-gallery) | `cargo run -p xpui-gallery -- --board x3`, and the seven slugs it takes |
| [Boards](docs/running.md#boards) | `Panel::of(board)`, and `Simulator::boards(&[..])` for the cycle `B` walks |
| [Keys and mouse](docs/running.md#keys-and-mouse) | what every key sends, and how a click becomes a tap |
| [Changing it while it runs](docs/running.md#changing-it-while-it-runs) | board, zoom, body, screenshot — without restarting |
| [The device around the panel](docs/running.md#the-device-around-the-panel) | the bezel drawn from millimetres, with clickable keys |
| [`--frames N`](docs/running.md#--frames-n) | why the loop is testable, and how CI runs it |
| [The mouse as a finger](docs/running.md#the-mouse-as-a-finger) | a drag is a swipe, the wheel is a scroll, and where the edge gestures live |
| [Headless](docs/running.md#headless) | screenshots with no window, and no window at all |
| [When it does not run](docs/running.md#when-it-does-not-run) | the failures people actually hit |

## Requirements

SDL2, which `embedded-graphics-simulator` needs.

```bash
brew install sdl2           # macOS
apt install libsdl2-dev     # Debian/Ubuntu
```

## Why the window redraws when nothing changed

E-ink takes a second or more to refresh, so `App` only repaints when something
actually changed. The window still has to be pumped every frame or the OS
thinks the app has hung — so the loop pushes the unchanged framebuffer and
sleeps, rather than spinning a core.
