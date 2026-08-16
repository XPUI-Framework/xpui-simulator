# Running the simulator

A window, an event pump and a keyboard map around the
[`embedded_graphics`](../../embedded_graphics/README.md) backend. There is no
simulator-specific drawing path — the pixels in the window are the pixels a
panel would get, from the same backend, the same components and the same font
metrics. [The README](../README.md) covers what the crate is; this covers
running it.

## SDL2

`embedded-graphics-simulator` links SDL2, so it has to be installed before
anything here builds.

| Platform | |
|---|---|
| macOS | `brew install sdl2` |
| Linux, WSL | `sudo apt install libsdl2-dev` |

Nothing else is needed. The simulator is a plain `cargo run`.

## Running the gallery

```bash
cargo run -p xpui-gallery
```

That is the fastest way to see a framework change: the
[gallery](../../../../examples/gallery/) has a screen each for controls, lists,
dialogs, scrolling and text, so a widget that broke shows up in one of them.

Opening a screen of your own instead:

```rust
use xpui_simulator::{Panel, Simulator};

fn main() {
    Simulator::new(Panel::PORTRAIT).title("my reader").run(MyScreen::new());
}
```

`Panel::PORTRAIT` is 480 × 800 and `Panel::LANDSCAPE` is 800 × 480, both at 1:1.
A 1-bit panel at 1:1 is hard to read on a high-density display, so
`Panel::PORTRAIT.scaled(2)` doubles every panel pixel in the window without
changing what the screen is laid out against.

## Boards

`Panel::of(board)` takes a [`Board`](../../chrome/src/board.rs) — a panel size,
the chrome sized for it, and whether it has a touchscreen. It is the *same*
value a firmware reads, which is what makes "develop in a window, then flash
it" true rather than aspirational, and it picks a sensible scale so a 296 × 128
strip is not a postage stamp on a modern display.

```bash
cargo run -p xpui-gallery -- --board badger2040
cargo run -p xpui-gallery -- --board tufty2040
cargo run -p xpui-gallery -- --board reader            # the default
cargo run -p xpui-gallery -- --board reader-landscape
```

This is worth doing early rather than at the end. A Badger 2040's content band
is 90 pixels; a screen that looks spacious at 480 × 800 can have nowhere to put
its third row, and the panel is where you find that out. With the *default*
chrome that band is 28 pixels and a list draws no rows at all, which is why
`Board` carries a token preset rather than only a size.

## Keys and mouse

Named by meaning rather than position, because that is the framework's own
contract: a screen asks for `Confirm` and the host decides what that is.

| Key | |
|---|---|
| Up / Down | `Button::Up` / `Button::Down` — move focus |
| Left / Right | `Button::Left` / `Button::Right` — nudge whatever holds focus |
| Enter, keypad Enter, Space | `Button::Confirm` |
| Backspace | `Button::Back` |
| Page Up / Page Down | `Button::PageBack` / `Button::PageForward` |
| H | the home gesture |
| Q, Escape, or closing the window | quit |

**Escape cannot be Back**, however much it ought to be.
`embedded-graphics-simulator` turns it into `SimulatorEvent::Quit` before the
event reaches us, so there is no key press left to interpret. Backspace is Back;
Escape closes the window.

| Mouse | |
|---|---|
| Left click | a tap |
| Left drag | held positions, frame by frame |
| Scroll wheel | a swipe: wheel up reports `SwipeDir::Down`, wheel down `SwipeDir::Up` |

So the touch paths are exercised too, not only the buttons.

Two details of the mouse are deliberate. A tap is reported at the point the
button went *down*, not where it came up, which is what the framework expects
and what stops a slightly shaky click landing on a different control. And a
release that drifted more than eight pixels from the press is not reported as a
tap at all — it was a drag, and the drag frames already went through.

## `--frames N`

```bash
cargo run -p xpui-gallery -- --frames 60
```

Stops after that many frames instead of waiting for the window to close.

This is what makes the simulator testable. The loop otherwise ends only when the
app's screen stack empties, when Q is pressed, or when the window is closed —
none of which happens on its own. A CI run, or any check that the loop even
*starts*, would hang until something killed it.

`--frames` is the gallery's own flag, parsed by hand in
[`examples/gallery/src/main.rs`](../../../../examples/gallery/src/main.rs) and
passed to `Simulator::frames`. An application embedding the simulator wires up
its own way of setting it, or none.

## Headless

```bash
SDL_VIDEODRIVER=dummy cargo run -p xpui-gallery -- --frames 30
```

`dummy` gives SDL a windowless target, so this works over ssh and on a CI runner
with no display. Combined with `--frames` it is a complete smoke test of the
loop, which is exactly what
[`examples/gallery/tests/simulator.rs`](../../../../examples/gallery/tests/simulator.rs)
does — it runs the binary headlessly for 30 frames and again for 1, and fails if
either panics or overruns a deadline.

That test exists because the simulator once died on startup every single time:
`Window::events()` panics if it is called before the first `update()`, and the
loop called it on its first iteration. The workspace built and every unit test
passed. Building is not running.

## No window at all

A window is the wrong tool for asserting on a screen. The `embedded_graphics`
backend's `framebuffer` feature renders to memory instead, with no SDL and no
simulator involved:

```rust
use xpui_eg::{Backend, Framebuffer, Palette};

let backend = Backend::leak(
    Framebuffer::new(480, 800),
    Palette::new(BinaryColor::On, BinaryColor::Off),
);
unsafe { xpui::host::install(backend) };

App::new(MyScreen::new()).render();

backend.with_display(|frame| {
    frame.write_bmp("my_screen");           // a BMP you can open
    println!("{}", frame.thumbnail(60));    // an ASCII view small enough to diff
    assert!(frame.ink_in(0, 0, 480, 56) > 0, "it has a header");
});
```

Input goes in the same way the simulator feeds it — `backend.begin_frame(ms)`,
`backend.press(Button::Confirm)`, `app.tick()` — so a test can press a button and
render what came back.
[`examples/gallery/tests/screenshots.rs`](../../../../examples/gallery/tests/screenshots.rs)
is the worked example: it shoots every gallery screen, compares an ASCII
thumbnail against a text golden, and writes the BMPs somewhere you can look at
them.

One trap in a workspace: an integration test's working directory is its own
crate root, which is *not* where the shared `target/` is. `write_bmp` lands
relative to the working directory, so a test that wants every shot in one place
passes `env!("CARGO_TARGET_TMPDIR")` to `write_bmp_in` instead, or sets
`XPUI_SCREENSHOT_DIR`.

## When it does not run

| | |
|---|---|
| SDL2 not found at link time | Install it (above) and build again. |
| No window on a remote session | Set `DISPLAY`, or run headless with `SDL_VIDEODRIVER=dummy`. |
| The window redraws while nothing changes | Expected. E-ink refreshes slowly, so the app only repaints when something changed — but SDL still has to be pumped every frame or the OS decides the app has hung. The loop pushes the unchanged frame and sleeps rather than spinning a core. |
