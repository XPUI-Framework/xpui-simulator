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
    Simulator::new(Panel::PORTRAIT)
        .title("my reader")
        .run(MainMenu::new());
}
```

Not a separate rendering backend: it is
[`xpui-embedded-graphics`](../embedded_graphics/) over
`embedded-graphics-simulator`'s display, plus a window, an event pump and a
keyboard mapping. **The pixels are the ones a device would get** — same
backend, same components, same measurements.

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

Clicking is a tap, dragging reports held positions, and the scroll wheel is a
swipe — so the touch paths are exercised too, not just the buttons.

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
