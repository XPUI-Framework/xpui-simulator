# Simulator

The window, the loop that drives it, and the session that loop keeps: which
board is on screen, at what scale, and with or without its body. `Simulator` is
what an application calls. `Session` is the same state without SDL, so a test
can switch boards and zoom with no window anywhere.

[Running the simulator](../running.md) is the guide: the keys, the mouse, the
flags and what to do when it does not start. This page is what each piece does.

## Topics

| | |
|---|---|
| [`Simulator`](#simulator) | A window, a backend, and the loop between them. |
| [`Session`](#session) | What the window is showing, and everything that can change about it. |
| [`window_settings`](#window_settings) | How the window presents one panel pixel. |
| [`open_frame`](#open_frame) | Opens a frame: the clock, the one-frame input, and anything held back. |
| [`capture_panel`](#capture_panel) | Writes the panel to `dir` and answers where it went. |

## `Simulator`

A window, a backend, and the loop between them.

```text
pub struct Simulator
```

A builder: every method but [`run`](#simulatorrun) and
[`session`](#simulatorsession) takes the simulator by value and hands it back.

| Builder | Sets | When not called |
|---|---|---|
| [`Simulator::new`](#simulatornew) | the panel the window opens on | — |
| [`boards`](#simulatorboards) | the cycle **B** and **Shift+B** walk | the panel's own board, alone |
| [`title`](#simulatortitle) | the window's title | `xpui` |
| [`keys`](#simulatorkeys) | what reads meaning into a press | [`Raw`](input.md#raw): presses go through untouched |
| [`frames`](#simulatorframes) | a frame limit | runs until the app finishes or the window closes |

**This crate knows no device.** The panel is always the caller's: `Board::custom`
describes any panel, and `xpui-boards-pimoroni`, `xpui-boards-xteink` and
`xpui-boards-seeed` carry ready-made ones. The examples on these pages use the
vendor crates, which an application adds as its own dependencies.

**Example — opening a window**

`no_run`, because `run` opens a window and pumps events until someone closes it.

```rust,no_run
# use xpui::{NavigationScreen, Screen, Text, View, vstack};
# struct Home;
# impl Screen for Home {
#     type Message = ();
#     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("hello")]) }
#     fn update(&mut self, _message: ()) {}
# }
use xpui_boards_xteink as xteink;
use xpui_simulator::{Panel, Simulator};

fn main() {
    Simulator::new(Panel::of(xteink::X4))
        .boards(&[xteink::X3, xteink::X4, xteink::X4_PRO])
        .title("reader")
        .run(Home);
}
```

**Example — what a simulator will open, without opening it**

```rust
use xpui_boards_xteink as xteink;
use xpui_simulator::{Panel, Simulator};

let simulator = Simulator::new(Panel::of(xteink::X4)).boards(&[xteink::X3, xteink::X4]);
let session = simulator.session();

assert_eq!(session.board(), xteink::X4, "it opens on the panel's board");
assert_eq!(session.boards(), &[xteink::X3, xteink::X4], "and walks the caller's cycle");
```

### Creating a simulator

#### `Simulator::new`

A simulator that opens on `panel`, cycling that one board, titled `xpui`, running until the window is closed, delivering presses raw.

```text
pub fn new(panel: Panel) -> Self
```

The window's scale comes from the panel: [`Panel::of`](panel.md#panelof) picks
one, and [`Panel::scaled`](panel.md#panelscaled) overrides it.

#### `Simulator::boards`

The boards the board keys cycle through, in order.

```text
pub fn boards(self, boards: &[Board]) -> Self
```

A list that leaves out the panel's own board gets it added at the end, so the
board on screen is always one the keys can come back to; see
[`Session::cycling`](#sessioncycling).

#### `Simulator::title`

The window's title.

```text
pub fn title(self, title: impl Into<String>) -> Self
```

#### `Simulator::keys`

Reads meaning into presses before the framework sees them.

```text
pub fn keys(self, keys: impl Keys + 'static) -> Self
```

A board with fewer keys than it has meanings has to fold two together, and
that is the firmware's decision, so it is the caller's here. See
[`Keys`](input.md#keys).

#### `Simulator::frames`

Stops after `frames` frames instead of waiting for the window to close.

```text
pub fn frames(self, frames: u32) -> Self
```

What makes the loop testable: with `SDL_VIDEODRIVER=dummy` and a frame limit,
a CI runner with no display runs the whole loop and exits. The gallery's
`--frames N` flag is its own, passed through to this; see
[`--frames N`](../running.md#--frames-n).

### Running it

#### `Simulator::session`

The [`Session`](#session) [`run`](#simulatorrun) will drive.

```text
pub fn session(&self) -> Session
```

Everything the builder was told, resolved, without a window. Building a
session installs its backend as the process's host, as `run` does.

#### `Simulator::run`

Runs `root` until the app finishes or the window closes.

```text
pub fn run<S: Screen + 'static>(self, root: S)
```

The loop ends when the app's screen stack empties, when **Q** or Escape is
pressed, when the window is closed, or after [`frames`](#simulatorframes)
frames. Each frame reads one clock value, opens the frame with
[`open_frame`](#open_frame), handles the window's events, ticks the app and
repaints only what changed.

**See also:** [`Session`](#session), [`Panel`](panel.md#panel), [`Keys`](input.md#keys)

## `Session`

What the window is showing, and everything that can change about it.

```text
pub struct Session
```

**The window never resizes.** A session sizes it once, for the largest board in
its cycle at that board's own scale, and letterboxes every smaller one into the
middle. Zoom and body are measured against that one size: a scale whose device
would not fit is refused, which is why a reader-sized panel stays at life size
and the small strips are where zoom buys something.

**One backend per board**, built the first time that board is shown and reused
after. Each is leaked, because `xpui::host::install` takes a `&'static`, so the
cost is bounded by the length of the cycle rather than by how often **B** is
pressed. Switching board installs that board's backend as the process's host.

Nothing in a session touches SDL, so it is what a test drives.

> [!NOTE]
> A session installs a process-wide host. Two tests building sessions at once
> share it: run them one at a time.

**Example — switching board and body**

```rust
use xpui_boards_pimoroni as pimoroni;
use xpui_boards_xteink as xteink;
use xpui_simulator::{Control, Panel, Session};

let mut session = Session::cycling(Panel::of(xteink::X4), &[xteink::X4, pimoroni::BADGER_2040]);
let window = session.window_size();

assert!(session.apply(Control::NextBoard));
assert_eq!(session.board(), pimoroni::BADGER_2040);
assert!(session.apply(Control::NextBoard), "the cycle wraps");
assert_eq!(session.board(), xteink::X4);
assert_eq!(session.backends_built(), 2, "the X4's backend was reused");

assert!(session.apply(Control::ToggleBody));
assert!(!session.body_shown());
assert!(session.layout().is_none(), "a hidden body has no layout");

assert!(!session.apply(Control::Screenshot), "a screenshot changes nothing shown");
assert_eq!(session.window_size(), window, "and nothing resized the window");
assert!(session.scale() <= session.ceiling());
```

### Opening a session

#### `Session::new`

Opens on `panel`, and installs the backend behind it.

```text
pub fn new(panel: Panel) -> Session
```

The cycle is that one board.

#### `Session::cycling`

Opens on `panel`, with `boards` as the cycle the board keys walk.

```text
pub fn cycling(panel: Panel, boards: &[Board]) -> Session
```

| Parameter | Meaning |
|---|---|
| `panel` | The board it opens on, and the scale. A scale above six opens at six, and zero at one. |
| `boards` | The cycle, in the caller's order. A list without `panel`'s board gets it added at the end. |

### What is on screen

#### `Session::boards`

The cycle the board keys walk, in order: what `B` will do, said at startup rather than found by pressing it.

```text
pub fn boards(&self) -> &[Board]
```

#### `Session::board`

The board on screen.

```text
pub fn board(&self) -> Board
```

#### `Session::scale`

How many window pixels one panel pixel occupies.

```text
pub fn scale(&self) -> u32
```

#### `Session::body_shown`

Whether the device body is being drawn.

```text
pub fn body_shown(&self) -> bool
```

A session opens with the body shown. A board with no body draws none either way.

#### `Session::backend`

The backend the framework is installed on, which is this board's.

```text
pub fn backend(&self) -> &'static Backend<PanelDisplay>
```

What a test presses keys on and reads the panel from:
[`Keypad`](input.md#keypad) and [`capture_panel`](#capture_panel) both take it.

#### `Session::backends_built`

How many backends have been built, and so how many have been leaked.

```text
pub fn backends_built(&self) -> usize
```

### Where the device sits

#### `Session::window_size`

The window, fixed for the whole session.

```text
pub fn window_size(&self) -> (i32, i32)
```

#### `Session::shown_size`

How much of the window the device fills: the body when it is shown, and the bare panel when it is not.

```text
pub fn shown_size(&self) -> (i32, i32)
```

#### `Session::origin`

Where the device's top-left corner sits, centred in the fixed window.

```text
pub fn origin(&self) -> Point
```

#### `Session::layout`

The body around the panel, placed in the window — or `None` when this board has no body or the body is hidden.

```text
pub fn layout(&self) -> Option<BezelLayout>
```

Already moved to [`origin`](#sessionorigin), so its window pixels are the
window's. See [`BezelLayout`](panel.md#bezellayout).

#### `Session::panel_offset`

Where the panel's top-left corner sits in the window.

```text
pub fn panel_offset(&self) -> Point
```

Where the panel display is placed, with or without a body.

### Changing it

#### `Session::apply`

Applies a control, and answers whether the window has to be rebuilt.

```text
pub fn apply(&mut self, control: Control) -> bool
```

`false` means nothing changed: zoom already at the ceiling, zoom out at life
size, or [`Control::Screenshot`](input.md#control), which is the loop's to take.
After every control the scale is clamped to the new [`ceiling`](#sessionceiling),
because a board with a larger body, or a body shown again, may not fit the
scale the last one had.

#### `Session::ceiling`

The largest scale this board may be shown at: the largest whose window still fits the one that was opened.

```text
pub fn ceiling(&self) -> u32
```

Read against what is shown now, so hiding the body of a small device in a
large shell raises it. Never above six, and never below one.

**See also:** [`Simulator::session`](#simulatorsession), [`Control`](input.md#control), [`Panel`](panel.md#panel)

## `window_settings`

How the window presents one panel pixel.

```text
pub fn window_settings(scale: u32) -> OutputSettings
```

`scale` window pixels per panel pixel, with no gap between pixels, and ink on
paper: an `On` pixel is drawn `#1A1A1A` on `#F2F2EE`. The spacing is set
explicitly, because `embedded-graphics-simulator`'s theme builder otherwise
adds a one-pixel gap, which doubles the window and halves every mouse
coordinate.

**Example — the settings the panel is drawn with**

```rust
use embedded_graphics_simulator::BinaryColorTheme;
use xpui_simulator::window_settings;

let settings = window_settings(3);
assert_eq!(settings.scale, 3);
assert_eq!(settings.pixel_spacing, 0, "no gap between panel pixels");
assert!(matches!(settings.theme, BinaryColorTheme::Custom { .. }), "ink on paper, not identity");
```

[`paint_body`](panel.md#paint_body) shows these settings composing the panel
over its body into an image.

## `open_frame`

Opens a frame: the clock, the one-frame input, and anything held back.

```text
pub fn open_frame(keypad: &mut Keypad, backend: &Backend<PanelDisplay>, now: u32)
```

It starts the backend's frame at `now`, then delivers whatever the keypad's
[`Keys::due`](input.md#keysdue) says has come due. A loop of your own driving a
[`Keypad`](input.md#keypad) calls this rather than repeating the two steps, so
the order stays the same.

> [!NOTE]
> A press that has come due is delivered before any of this frame's events.
> It was held back from an earlier frame, so it happened first.

**Example — a press from a timer**

```rust
use xpui::Button;
use xpui::host::Input;
use xpui_boards_xteink as xteink;
use xpui_simulator::{Keypad, Keys, Panel, Session, open_frame};

/// Presses Confirm once, 300 ms into the run.
struct Later {
    fired: bool,
}

impl Keys for Later {
    fn due(&mut self, now: u32) -> Option<Button> {
        if self.fired || now < 300 {
            return None;
        }
        self.fired = true;
        Some(Button::Confirm)
    }
}

let session = Session::new(Panel::of(xteink::X4));
let mut keypad = Keypad::new(Box::new(Later { fired: false }));

open_frame(&mut keypad, session.backend(), 0);
assert!(!Input::was_pressed(Button::Confirm));

open_frame(&mut keypad, session.backend(), 300);
assert!(Input::was_pressed(Button::Confirm), "delivered as a press and its release");
```

## `capture_panel`

Writes the panel to `dir` and answers where it went.

```text
pub fn capture(backend: &Backend<PanelDisplay>, slug: &str, dir: &Path) -> PathBuf
```

Declared as `capture` and exported as `capture_panel`. It is what the **S** key
calls, with the board's slug and `$XPUI_SCREENSHOT_DIR`, or
`target/screenshots` when that is unset.

The file is a 1-bit BMP of the panel alone, never the body around it, named
`<slug>-<n>.bmp` with `n` the first number not already taken. A second capture
never overwrites the first, in the same run or a later one.

**Example — two captures of one panel**

```rust
use xpui_boards_xteink as xteink;
use xpui_simulator::{Panel, Session, capture_panel};

let session = Session::new(Panel::of(xteink::X3));
let dir = std::env::temp_dir().join("xpui-simulator-capture-example");
# let _ = std::fs::remove_dir_all(&dir);
std::fs::create_dir_all(&dir).expect("a directory to write to");

let first = capture_panel(session.backend(), session.board().slug, &dir);
let second = capture_panel(session.backend(), session.board().slug, &dir);
assert_eq!(first, dir.join("x3-1.bmp"));
assert_eq!(second, dir.join("x3-2.bmp"));
# std::fs::remove_dir_all(&dir).expect("the example cleans up");
```
