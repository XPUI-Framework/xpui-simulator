# Reference

The whole public API of `xpui-simulator`, by area.
[Running the simulator](running.md) is the guide: how to open a window, what
every key and click does, and how CI runs the loop. This is what you reach for
once you know the shape and want to know what exists.

## Topics

Each page lists every public name in its area with its declaration, what it
does and examples you can copy. The gate checks every page against the code,
so what a page says an item is, rustdoc agrees with, and every `rust` block is
compiled and run by `cargo test --doc`. None of them opens a window: the one
that would is marked `no_run`.

| Page | Holds |
|---|---|
| [simulator](reference/simulator.md) | `Simulator`, `Session`, `window_settings`, `open_frame`, `capture_panel`: the window, its loop, and the state behind them |
| [panel](reference/panel.md) | `Panel`, `PanelDisplay`, `BezelLayout`, `paint_body`, and the re-exported `Board` |
| [input](reference/input.md) | `Keys`, `Raw`, `Press`, `Keypad`, `button_for`, `Control`, `control_for`, `Touchscreen`, `Touch`, `Touches`, `EdgeGesture`, `Hit`, `route`, and the re-exported `Keycode` |

## Where the API documentation lives

> [!NOTE]
> **[docs.rs](https://docs.rs/) may have no page for this crate.** [`embedded-graphics-simulator`](https://crates.io/crates/embedded-graphics-simulator)
> links [SDL2](https://www.libsdl.org/), and docs.rs builds without it, so a docs.rs build of
> `xpui-simulator` fails. Until the window is behind a feature a docs.rs build
> can leave off, these pages are the reference, checked name for name against
> rustdoc. With SDL2 installed, `cargo doc -p xpui-simulator --open` in a clone
> builds the same documentation locally.
