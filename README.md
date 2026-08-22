# `xpui-simulator`

Runs an [`xpui`](../../xpui/) app in a desktop window, so screens can be
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
    Simulator::new(Panel::of(Board::X4))
        .title("my reader")
        .run(MainMenu::new());
}
```

Not a separate rendering backend: it is
[`xpui-embedded-graphics`](../embedded_graphics/) over
`embedded-graphics-simulator`'s display, plus a window, an event pump and a
keyboard mapping. **The pixels are the ones a device would get** — same
backend, same components, same measurements.

## The device around the panel

A [board](../../boards/) that has described its body opens a window larger than
its panel: the panel is inset into a shell, and the device's real buttons are
drawn where a thumb would find them — clickable, feeding the same input the
keyboard does. The Badger's five are along the front; the X3's Up and Down are
on the **side**. It is drawn from the published millimetre dimensions rather
than from a photograph, so zooming changes how big the device looks and nothing
about where anything sits on it.

Nothing the firmware draws can leave the panel rectangle, and **nothing outside
it becomes a touch** — a device with no touchscreen would otherwise receive one
at a coordinate it has no way of producing.

The row reading `Back OK Up Dn` *inside* the canvas is not one of these. That is
firmware UI, which the real device draws on the e-ink too.

A board that has not described a body shows the bare panel instead, letterboxed
in the middle of the window. Bodies arrive one device at a time.

## Keys

| Key | |
|---|---|
| Up / Down | move focus |
| Left / Right | nudge whatever holds focus |
| Enter or Space | confirm |
| Backspace | back |
| Page Up / Page Down | page |
| H | the home gesture |
| Q or Escape | quit |

**Escape cannot be Back.** `embedded-graphics-simulator` turns it into
`SimulatorEvent::Quit` before the event reaches the keyboard map, so there is no
key press left to interpret. Backspace is Back.

Clicking a physical button on the shell presses it, exactly as a key does, and
holding one shows it held.

**Holding a key sends one press.** The window manager's auto-repeat is dropped,
because hardware has none: a key held on a device sends one press and stays
down, and the framework runs its own repeat off that.

Keys arrive raw. What two presses close together *mean* is the firmware's
decision, so `Simulator::keys` hands them over before the framework sees them —
`translate` renames or swallows a press, `due` invents one from a timer. That is
how a board with fewer keys than jobs finds a Back it has no room for. See [`docs/running.md`](docs/running.md).

## Changing it while it runs

| Key | |
|---|---|
| B, Shift+B | the next board in the cycle you supplied, or the previous one |
| + / - | zoom in and out |
| E | show or hide the device body |
| S | write the panel to `target/screenshots/` |

**`B` walks the cycle you supplied.** This crate has no device list of its own:
`Simulator::boards(&[..])` is how you give it one, and the snippet above gives
it none, so `B` does nothing there. `examples/gallery/src/main.rs` passes seven.

**The screen stack survives a board switch**: the screen you had navigated to is
re-measured against the new panel and painted on it, which is the point — every
panel without a run each, and without navigating back each time.

The window itself never resizes; it is opened once for the largest board and
letterboxes the rest. That is what limits zoom, too. See
[docs/running.md](docs/running.md).

## The mouse as a finger

Clicking *on the panel* is a tap, dragging is a drag and then a swipe, holding
still is a long press, and a swipe that starts at an edge is a back, home or
menu gesture. The classification is CrossPoint's, ported constant for constant
from the firmware — the same slops, the same 700 ms swipe window, the same edge
bands — so a gesture that works here works on the device, and one the device
would refuse is refused here. The scroll wheel is a plain swipe as well, which
is how a board with no touchscreen still exercises the swipe paths.

**A board with no touchscreen reports no touch at all.** Clicking a Badger
2040's panel does nothing, because a Badger 2040 cannot be touched.

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
