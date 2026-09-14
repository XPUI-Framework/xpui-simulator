# Running the simulator

A window, an event pump and a keyboard map around the
[`embedded_graphics`](https://github.com/XPUI-Framework/xpui-backends/blob/main/embedded_graphics/README.md) backend. There is no
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

**The gallery is a repository of its own**, because it is the application and
this is the library it draws through. Clone it beside this one:

```bash
cd .. && git clone https://github.com/XPUI-Framework/xpui-gallery
```

Every block below opens by entering it, so each one stands on its own — this
page is read by jumping to a section, not from the top.

```bash
cd ../xpui-gallery
cargo run -p xpui-gallery
```

That is the fastest way to see a framework change: the
[gallery](https://github.com/XPUI-Framework/xpui-gallery/tree/main/gallery) has a screen each for controls, lists,
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
use xpui_simulator::{Board, Panel, Simulator};

fn main() {
    // Your panel. The vendor crates carry ready-made ones; this crate knows
    // no device and opens whatever it is handed.
    let mine = Board::custom("my reader", 480, 800, false);
    Simulator::new(Panel::of(mine)).title("my reader").run(MyScreen::new());
}
```

There is no default panel: this crate knows no devices, so a caller names one.
`Board::custom` describes any panel; `xpui-boards-pimoroni`, `xpui-boards-xteink`
and `xpui-boards-seeed` carry ready-made ones — a project depends on the vendor
it targets and not the other two.

`Panel::of(board)` gives that device its own size and a scale that keeps the
window within reach of a laptop display. A 480 × 800 reader is big enough to
show at 1:1; a 296 × 128 strip at 1:1 is a postage stamp, so it is tripled.

The board keys walk one board by default: the one the panel was opened on.
`Simulator::boards(&[..])` is how an application offers more, in its own order.
`xpui-gallery`'s `gallery/src/main.rs` passes all seven.

`Panel::of(..).scaled(n)` overrides that. Scale is a window concern: doubling
every panel pixel changes nothing about what the screen is laid out against.
Every field and method is in [the panel reference](reference/panel.md#panel).

## Boards

`Panel::of(board)` takes a [`Board`](https://github.com/XPUI-Framework/xpui-boards/blob/main/core/src/lib.rs) — a panel
size, what its keys mean, whether it has a touchscreen, and how much larger
than the baseline its chrome should be. Not the chrome itself: whoever wires
the backend derives that from the panel's size and that scale. It is the *same*
value a firmware reads, which is what makes "develop in a window, then flash
it" true rather than aspirational, and it picks a sensible scale so a 296 × 128
strip is not a postage stamp on a modern display.

```bash
cd ../xpui-gallery
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

The third method, `reset`, and [`Keypad`](reference/input.md#keypad), which
drives all three with no window, are in [the input reference](reference/input.md#keys).

The gallery installs a reader for both, though no board here needs it: every
one has a Back key of its own, so presses pass straight through. On a board
with three keys and no spare, the reader swallows the first press of the
stand-in key and re-issues it when the double-press window shuts — which costs
every select a third of a second. See `gallery::chord` for what to weigh before
choosing that arrangement.

| Mouse | |
|---|---|
| Left click on the panel | a tap |
| Left drag on the panel | held positions, then a swipe or a gesture on release |
| Left press, held still | classified as a long press after half a second, which nothing receives yet |
| Left click on a physical button | presses it, exactly as its key does |
| Scroll wheel | a swipe: wheel up reports `SwipeDir::Down`, wheel down `SwipeDir::Up` |

## Changing it while it runs

These keys change the simulator instead of restarting it, and the screen stack
survives all of them.

| Key | |
|---|---|
| B | the next board |
| Shift+B | the previous board |
| + | zoom in, as far as the window allows |
| - | zoom out, down to life size |
| E | show or hide the device body |
| S | write the panel to `target/screenshots/` and print the path |

Each is a [`Control`](reference/input.md#control), applied by
[`Session::apply`](reference/simulator.md#sessionapply).

Switching board installs a different backend, because a different panel size
needs one. The screens carry on running on it: the app owns the stack, and the
screen on top is re-measured against the new panel on the next frame. That is
the whole point — the same screen, on another panel, without navigating back
to it each time.

One backend is kept per board and reused, so cycling for an hour costs under
two megabytes in total rather than a panel's worth of pixels per press.

### The window never resizes

The window is opened once, large enough for the largest board these keys can
reach, and every smaller one is letterboxed into the middle of it;
[design.md](design.md) says why it cannot resize. Hiding the body does not
shrink the window — it grows the letterbox.

That is also what limits zoom. A scale whose device would not fit the window is
refused, and a 480 × 800 reader inside its body is already 1165 pixels tall, so
those boards stay at life size. Zoom is for the small panels: a Badger 2040
opens tripled and a Tufty 2040 doubled. In the window the gallery's seven
open, hiding the body takes the Tufty one step further; the Badger stays at
three, because its bare panel at four times is wider than any window it opens
in.

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
[`capture_panel`](reference/simulator.md#capture_panel) writes it, and a test
can call it with no window.

## The mouse as a finger

A click and drag goes through CrossPoint's touch model, ported constant for
constant from the firmware's `InputManager`; why a port rather than a model of
the simulator's own is in [design.md](design.md).

| | |
|---|---|
| A tap | up to **59 px** of travel, reported at the point the finger went **down** |
| A swipe | **60 px** on either axis, within **700 ms**, resolved to its dominant axis |
| A long press | **500 ms** still, cancelled by **28 px** of movement |
| Back | a right swipe starting in the left **25%** |
| Home | an up swipe starting in the bottom **14%** |
| Menu | a down swipe starting in the top **14%** |

A long press and Menu are classified and then go nowhere: the framework's input
has no slot for either yet. What each classification reaches is in
[`Touch`](reference/input.md#touch).

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
[`Bezel`](https://github.com/XPUI-Framework/xpui-boards/blob/main/core/src/bezel.rs) — is drawn inside it. The panel is
inset into a shell drawn from the device's published millimetre dimensions,
with its real buttons where a thumb would find them.

```bash
cd ../xpui-gallery
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

**Nothing a screen draws can leave the panel rectangle.** The backend wraps
every draw in `DrawTargetExt::clipped`, so a widget that measured itself wrong
is cut off at the panel's edge rather than painted over the bezel — which is
what a device would do, and what makes the body around the panel safe to draw
at all.

## `--frames N`

```bash
cd ../xpui-gallery
cargo run -p xpui-gallery -- --frames 60
```

Stops after that many frames instead of waiting for the window to close.

This is what makes the simulator testable. The loop otherwise ends only when the
app's screen stack empties, when Q is pressed, or when the window is closed —
none of which happens on its own. A CI run, or any check that the loop even
*starts*, would hang until something killed it.

`--frames` is the gallery's own flag, parsed by hand in
[`xpui-gallery`'s `gallery/src/main.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/src/main.rs) and
passed to `Simulator::frames`. An application embedding the simulator wires up
its own way of setting it, or none.

## Headless

```bash
cd ../xpui-gallery
SDL_VIDEODRIVER=dummy cargo run -p xpui-gallery -- --frames 30
```

`dummy` gives SDL a windowless target, so this works over ssh and on a CI runner
with no display. Combined with `--frames` it is a complete smoke test of the
loop, which is exactly what
[`xpui-gallery`'s `gallery/tests/simulator.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/tests/simulator.rs)
does — it runs the binary headlessly for 30 frames and again for 1, and fails if
either panics or overruns a deadline.

That test exists because the simulator once died on startup every single time:
`Window::events()` panics if it is called before the first `update()`, and the
loop called it on its first iteration. The workspace built and every unit test
passed. Building is not running.

## No window at all

A window is the wrong tool for asserting on a screen. `xpui-screenshot`
renders to memory instead, with no SDL and no simulator involved:

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
use xpui_eg::{Backend, Palette};
use xpui_screenshot::{Framebuffer, assert_screenshot};

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
[`xpui-gallery`'s `gallery/tests/typeface.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/tests/typeface.rs)
is the shortest worked example of the assertion above.
[`xpui-gallery`'s `gallery/tests/screenshots.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/tests/screenshots.rs)
is the fuller one: it pairs goldens with `ink_in` checks like the one beside
the assertion above, and runs the pair across the gallery's seven — nine screens on
seven panels, each against a PNG committed as `<screen>_<board slug>.png`,
pixel for pixel. That one calls `check_screenshot` rather than
`assert_screenshot`, the same comparison handing its report back instead of
panicking with it, so a run names every board that moved rather than the first.
A mismatch writes `target/diff/<name>.png` either way — expected, actual and
the differences, side by side.

`frame.write_bmp("name")` and `frame.thumbnail(60)` are still there for looking
at a frame that has no golden. Nothing compares either of them, so do not pair
one with a screenshot assertion: two artifacts where only one is authoritative
is how the other one ends up trusted.

## When it does not run

| | |
|---|---|
| SDL2 not found at link time | Install it (above) and build again. |
| No window on a remote session | Set `DISPLAY`, or run headless with `SDL_VIDEODRIVER=dummy`. |
| The window redraws while nothing changes | Expected; [design.md](design.md) says why. |
