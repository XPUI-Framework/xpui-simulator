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

```rust,no_run
# use xpui::{NavigationScreen, Screen, Text, View, vstack};
# struct MyScreen;
# impl MyScreen { fn new() -> Self { MyScreen } }
# impl Screen for MyScreen {
#     type Message = ();
#     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("hello")]) }
#     fn update(&mut self, _: ()) {}
# }
use xpui_simulator::{Panel, Simulator};

fn main() {
    Simulator::new(Panel::DEFAULT).title("my reader").run(MyScreen::new());
}
```

`Panel::DEFAULT` is the X4: 800 × 480 at 1:1. `Panel::of(board)` gives any
other device its own size and a scale that keeps the window within reach of a
laptop display — a 296 × 128 strip at 1:1 is a postage stamp, so it is tripled.

`Panel::of(..).scaled(n)` overrides that. Scale is a window concern: doubling
every panel pixel changes nothing about what the screen is laid out against.

## Boards

`Panel::of(board)` takes a [`Board`](../../chrome/src/board.rs) — a panel size,
the chrome sized for it, and whether it has a touchscreen. It is the *same*
value a firmware reads, which is what makes "develop in a window, then flash
it" true rather than aspirational, and it picks a sensible scale so a 296 × 128
strip is not a postage stamp on a modern display.

```bash
cargo run -p xpui-gallery -- --board badger2040
cargo run -p xpui-gallery -- --board tufty2040
cargo run -p xpui-gallery -- --board x4            # the default
cargo run -p xpui-gallery -- --board sticky
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
| Left click on the panel | a tap |
| Left drag on the panel | held positions, then a swipe or a gesture on release |
| Left press, held still | a long press after half a second |
| Left click on a physical button | presses it, exactly as its key does |
| Scroll wheel | a swipe: wheel up reports `SwipeDir::Down`, wheel down `SwipeDir::Up` |

## The mouse as a finger

A click and drag goes through CrossPoint's touch model, ported constant for
constant from the firmware's `InputManager`, so a gesture that works in this
window works on the device and one the device would refuse is refused here.

| | |
|---|---|
| A tap | up to **59 px** of travel, reported at the point the finger went **down** |
| A swipe | **60 px** on either axis, within **700 ms**, resolved to its dominant axis |
| A long press | **500 ms** still, cancelled by **28 px** of movement |
| Back | a right swipe starting in the left **25%** |
| Home | an up swipe starting in the bottom **14%** |
| Menu | a down swipe starting in the top **14%** |

Two of those numbers look like typos and are not. A tap survives 59 px while a
long press dies at 28, because they answer different questions: one asks
whether the finger was ever *still*, the other whether it went far enough to
have meant somewhere else. They were once the same number, and the 29..59 px
gap that left — where an ordinary finger roll was neither a tap nor a swipe —
is the reason they are not. A tap is reported where the finger landed for a
related reason: the contact point drifts 10-20 px as a finger rolls off during
a lift, and routing the release point makes small targets feel unreliable.

**A click that missed the panel is never a touch**, and neither is a click on a
board whose `touch` is `false` — a Badger 2040 has no touchscreen, so clicking
its panel does nothing at all. A device cannot receive a touch at a coordinate
it has no way of producing, and a simulator that invented one would let a
screen ship depending on it. Use the keys, or a board that has a touchscreen.

## The device around the panel

A board that has described its body — see
[`Bezel`](../../../boards/src/bezel.rs) —
opens a window larger than its panel. The panel is inset into a shell drawn from
the device's published millimetre dimensions, with its real buttons where a thumb
would find them.

```bash
cargo run -p xpui-gallery -- --board badger2040   # five buttons, all on the front
cargo run -p xpui-gallery -- --board x3           # Up and Down on the side
```

Clicking one presses it and holding one shows it held, which is worth doing
early for the same reason picking the right board is: it is how you notice that
the Badger's five buttons are the *only* input it has, and that a screen built
around a fifth control has nowhere to put it.

The layout is kept in tenths of a millimetre rather than pixels, so the scale
the window opens at changes how big the device is drawn and nothing about where
anything sits on it. That scale is now chosen from the whole window rather than
from the panel, because a small panel can sit in a comparatively large body.

The row reading `Back OK Up Dn` *inside* the canvas is not one of these
buttons. It is firmware UI, which the real device draws on the e-ink too.

A board that has not described a body opens a window that is exactly the panel,
as before — the X4 and the Sticky both do today.

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

```rust,no_run
# use embedded_graphics::pixelcolor::BinaryColor;
# use xpui::App;
# use xpui::{NavigationScreen, Screen, Text, View, vstack};
# struct MyScreen;
# impl MyScreen { fn new() -> Self { MyScreen } }
# impl Screen for MyScreen {
#     type Message = ();
#     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("hello")]) }
#     fn update(&mut self, _: ()) {}
# }
use xpui_eg::{assert_screenshot, Backend, Framebuffer, Palette};

let backend = Backend::leak(
    Framebuffer::new(480, 800),
    Palette::new(BinaryColor::On, BinaryColor::Off),
);
unsafe { xpui::host::install(backend) };

App::new(MyScreen::new()).render();

backend.with_display(|frame| {
    assert_screenshot("my_screen", frame);                  // against a committed PNG
    assert!(frame.ink_in(0, 0, 480, 56) > 0, "it has a header");
});
```

Input goes in the same way the simulator feeds it — `backend.begin_frame(ms)`,
`backend.press(Button::Confirm)`, `app.tick()` — so a test can press a button and
render what came back.
[`examples/gallery/tests/screenshots.rs`](../../../../examples/gallery/tests/screenshots.rs)
is the worked example: it shoots every gallery screen and compares each one
against a PNG committed in `tests/screenshots/`, pixel for pixel. A mismatch
writes `target/diff/<name>.png` — expected, actual and the differences, side by
side.

`frame.write_bmp("name")` and `frame.thumbnail(60)` are still there for looking
at a frame that has no golden. Nothing compares either of them, so do not pair
one with a screenshot assertion: two artifacts where only one is authoritative
is how the other one ends up trusted.

## When it does not run

| | |
|---|---|
| SDL2 not found at link time | Install it (above) and build again. |
| No window on a remote session | Set `DISPLAY`, or run headless with `SDL_VIDEODRIVER=dummy`. |
| The window redraws while nothing changes | Expected. E-ink refreshes slowly, so the app only repaints when something changed — but SDL still has to be pumped every frame or the OS decides the app has hung. The loop pushes the unchanged frame and sleeps rather than spinning a core. |
