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
use xpui_simulator::{Panel, Simulator};

fn main() {
    Simulator::new(Panel::DEFAULT)
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

A board that has not described a body opens a window that is exactly the panel,
as before. Bodies arrive one device at a time.

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

Clicking *on the panel* is a tap, dragging reports held positions, and the
scroll wheel is a swipe — so the touch paths are exercised too, not just the
buttons. Clicking a physical button on the shell presses it, exactly as a key
does, and holding one shows it held.

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
