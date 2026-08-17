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
dialogs, scrolling, text, typefaces and one that puts everything on a single
page, so a widget that broke shows up in one of them.

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

`Panel::of(board)` takes a [`Board`](../../../boards/src/lib.rs) — a panel size,
the chrome sized for it, and whether it has a touchscreen. It is the *same*
value a firmware reads, which is what makes "develop in a window, then flash
it" true rather than aspirational, and it picks a sensible scale so a 296 × 128
strip is not a postage stamp on a modern display.

```bash
cargo run -p xpui-gallery -- --board x4            # the default
cargo run -p xpui-gallery -- --board x3
cargo run -p xpui-gallery -- --board x4pro         # the touch reader
cargo run -p xpui-gallery -- --board sticky
cargo run -p xpui-gallery -- --board badger2040
cargo run -p xpui-gallery -- --board tufty2040
cargo run -p xpui-gallery -- --board inkyframe
```

`--board` picks the one it opens on. **B** walks the rest of them without
restarting — see [Changing it while it runs](#changing-it-while-it-runs) —
which is the quicker way to see a screen on all of them.

This is worth doing early rather than at the end. A Badger 2040's content band
is 90 pixels; a screen that looks spacious at 480 × 800 can have nowhere to put
its third row, and the panel is where you find that out. With the *default*
chrome that band is 28 pixels and a list draws no rows at all, which is why
`Board` carries a token preset rather than only a size.

### The window lies about how big everything is

A reader's panel is 217 to 257 ppi. A laptop display is around 110, so the
window shows it at roughly twice life size: a label that looks generous here is
a shade over 2mm on the glass, and a row that looks like a comfortable target
is 4mm across.

Nothing in the window can tell you that, so the boards carry the panel's
diagonal and `Board::tenths_of_a_mm` answers in millimetres. A board a finger
drives also carries a UI scale — its chrome and its type come out larger than
the same panel's would with keys, which is why an X4 and a touch reader of the
same size do not look alike in the window either.

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

**Holding a key sends one press.** The window manager's auto-repeat is dropped,
because hardware has none: a key held on a device sends one press and stays
down, and the framework runs its own repeat off that. Letting the repeats
through re-arms that timer on each one, so a held key would step at the desktop's
rate rather than the framework's.

### Reading something into a press

Keys reach the framework as the hardware sent them. A board can have fewer keys
than it has meanings — a badge with three along its bottom edge has no room for
a Back key — and folding two together is the *firmware's* decision, not the
simulator's. `Simulator::keys` is where a caller gets between:

```rust
# use xpui::Button;
# use xpui_simulator::{Keys, Press};
/// Swaps the page keys over, for somebody holding the device the other way up.
struct Swapped;

impl Keys for Swapped {
    fn translate(&mut self, press: Press) -> Option<Button> {
        Some(match press.button {
            Button::PageBack => Button::PageForward,
            Button::PageForward => Button::PageBack,
            other => other,
        })
    }
}
```

Two directions, and they are not the same thing:

- **`translate`** renames a press. Returning `None` swallows it. The release of
  that key follows whatever it became, so a press turned into `Back` is released
  as `Back` — a caller never has to track that.
- **`due`** invents one, from a timer, with no key behind it. It is delivered as
  a press *and* its release in the same frame, because there is no finger to
  lift later and a button left held auto-repeats.

The gallery uses both: on a three-key board it swallows the first press of `a`
and re-issues it when the double-press window shuts. That costs every select on
those boards a third of a second — see `gallery::chord` for why it is worth it
there and what to weigh before choosing it for another board.

| Mouse | |
|---|---|
| Left click on the panel | a tap |
| Left drag on the panel | held positions, then a swipe or a gesture on release |
| Left press, held still | a long press after half a second |
| Left click on a physical button | presses it, exactly as its key does |
| Scroll wheel | a swipe: wheel up reports `SwipeDir::Down`, wheel down `SwipeDir::Up` |

## Changing it while it runs

Checking a screen on every panel used to be one `cargo run` per board, each
losing whatever you had navigated to — and that state is usually the thing you
wanted to look at. These keys change the simulator instead of restarting it,
and the screen stack survives all of them.

| Key | |
|---|---|
| B | the next board |
| Shift+B | the previous board |
| + | zoom in, as far as the window allows |
| - | zoom out, down to life size |
| E | show or hide the device body |
| S | write the panel to `target/screenshots/` and print the path |

Switching board installs a different backend, because a different panel size
needs one. The screens carry on running on it: the app owns the stack, and the
screen on top is re-measured against the new panel on the next frame. That is
the whole point — the same screen, on another panel, without navigating back
to it each time.

One backend is kept per board and reused, so cycling for an hour costs under
two megabytes in total rather than a panel's worth of pixels per press.

### The window never resizes

There is no resize API: `MultiWindow` fixes its SDL window and its streaming
texture in the constructor. So the window is opened once, large enough for the
largest board these keys can reach, and every smaller one is letterboxed into
the middle of it. Hiding the body does not shrink the window — it grows the
letterbox.

That is also what limits zoom. A scale whose device would not fit the window is
refused, and a 480 × 800 reader inside its body is already 1165 pixels tall, so
those boards stay at life size. Zoom is for the small panels: a Badger 2040
opens tripled, a Tufty 2040 doubled, and both go further with the body hidden.

**Zoom changes nothing about the layout.** Scale is a window concern — the
panel is the same number of pixels at 1× as at 3× — and a screen that
re-lays-out when you zoom means the scale has leaked into the board.

### Screenshots

`S` writes the *panel*: not the window, and not the device drawn around it. A
picture of a screen with a simulated body in it is not a picture of what the
device would show.

The file is a 1-bit BMP named `<slug>-<n>.bmp`, and its path is printed — a
screenshot you cannot find is not a screenshot. `n` is the first number not
already taken, so pressing the key twice gives two files and a later run does
not overwrite an earlier one's. `XPUI_SCREENSHOT_DIR` moves them elsewhere.

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
[`Bezel`](../../../boards/src/bezel.rs) — is drawn inside it. The panel is
inset into a shell drawn from the device's published millimetre dimensions,
with its real buttons where a thumb would find them.

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
anything sits on it. That scale is chosen from the whole window rather than
from the panel, because a small panel can sit in a comparatively large body.

The row reading `Back OK Up Dn` *inside* the canvas is not one of these
buttons. It is firmware UI, which the real device draws on the e-ink too.

**E** hides the body, leaving the bare panel letterboxed in the middle of the
window — which is what a board that has never described one shows.

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
