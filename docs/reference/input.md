# Keys and touch

What the keyboard and the mouse become before the framework sees them. A key is
a `Button` the hardware sent, handed to the caller's [`Keys`](#keys) to read
meaning into. A click is routed to the panel, a physical key or bare shell, and
a click on the panel goes through [CrossPoint](https://crosspointreader.com/)'s touch model, ported constant for
constant. The control keys change the window rather than the screen.

Every piece here runs without a window: the event loop is the only part that
needs [SDL](https://www.libsdl.org/), and it does nothing to a press or a click that these do not.

[Keys and mouse](../running.md#keys-and-mouse) and
[The mouse as a finger](../running.md#the-mouse-as-a-finger) are the guide,
with every key's meaning and every touch threshold. This page is what each
piece does.

## Topics

| | |
|---|---|
| [`Keys`](#keys) | Reads meaning into raw presses. |
| [`Raw`](#raw) | Presses reach the framework as they arrive: what a board with a key for everything wants. |
| [`Press`](#press) | A key going down, and what was true when it did. |
| [`Keypad`](#keypad) | The keys of a device, with a `Keys` reading them. |
| [`button_for`](#button_for) | What a key means. |
| [`Control`](#control) | What a control key does to the window. |
| [`control_for`](#control_for) | What a key means, or `None` when it is not a control at all. |
| [`Touchscreen`](#touchscreen) | A touchscreen, and whatever is currently on it. |
| [`Touch`](#touch) | What one raw event turned into. |
| [`Touches`](#touches) | The events one raw event produced, in the order they happened. |
| [`EdgeGesture`](#edgegesture) | A swipe that means more than its direction, because of where it began. |
| [`Hit`](#hit) | What a click turned out to be. |
| [`route`](#route) | Where a click at `window_point` goes. |
| [`Keycode`](#re-exports) | Re-exported from SDL2, through `embedded-graphics-simulator`. |

## `Keys`

Reads meaning into raw presses.

```text
pub trait Keys
```

Presses arrive as the hardware sent them. A board with fewer keys than it has
meanings, such as a badge with three keys and no Back, has to fold two
together, and what two presses mean is the firmware's decision. An
implementation is where a caller makes it, installed with
[`Simulator::keys`](simulator.md#simulatorkeys).

Two directions, and they are not the same thing:

- **A translation**, [`translate`](#keystranslate), renames a key that went
  down. Its release follows what it became: a press translated to `Back` is
  released as `Back`.
- **An injection**, [`due`](#keysdue), is a press with no key behind it, from a
  timer. It is delivered as a press and its release in one frame, because there
  is no finger to lift later.

Every method has a default that does nothing, so an implementation states only
what it changes. [`Raw`](#raw) implements it with the defaults.

**Example — swapping the page keys**

```rust
use xpui::Button;
use xpui::host::Input;
use xpui_boards_xteink as xteink;
use xpui_simulator::{Keypad, Keys, Panel, Press, Session};

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

let session = Session::new(Panel::of(xteink::X4));
let mut keypad = Keypad::new(Box::new(Swapped));

session.backend().begin_frame(0);
keypad.down(session.backend(), Button::PageBack, 0, session.board());
assert!(Input::is_pressed(Button::PageForward), "what went down is the translation");

session.backend().begin_frame(16);
keypad.up(session.backend(), Button::PageBack);
assert!(!Input::is_pressed(Button::PageForward), "and it comes up with the key");
```

[`open_frame`](simulator.md#open_frame) has an example of an injection.

### Provided methods

#### `Keys::translate`

What `press` means, or `None` to swallow it.

```text
fn translate(&mut self, press: Press) -> Option<Button>
```

Whatever comes back is pressed, and released when the same physical key comes
up. A swallowed press is gone, unless [`due`](#keysdue) produces one later. The
default returns `press.button`.

#### `Keys::due`

A press that has come due, with no key behind it.

```text
fn due(&mut self, now: u32) -> Option<Button>
```

Asked once a frame, whether or not anything was pressed. The default returns
`None`.

#### `Keys::reset`

Everything held is being let go of, and nothing pending should arrive.

```text
fn reset(&mut self)
```

Called when the simulator switches board under a running app, so a press held
back on a device that is no longer on screen does not land on the one that
replaced it. Hardware has no equivalent. The default does nothing.

**See also:** [`Keypad`](#keypad), [`Press`](#press), [`Simulator::keys`](simulator.md#simulatorkeys)

## `Raw`

Presses reach the framework as they arrive: what a board with a key for everything wants.

```text
pub struct Raw
```

What a [`Simulator`](simulator.md#simulator) reads presses with until
[`keys`](simulator.md#simulatorkeys) says otherwise.

## `Press`

A key going down, and what was true when it did.

```text
pub struct Press
```

| Field | |
|---|---|
| `Press::button` | The button the hardware sent — the key's own identity, before anybody reads anything into it. |
| `Press::now` | Milliseconds since the run started, the same reading the framework is given this frame. |
| `Press::board` | The device this press came from. |

A struct rather than three arguments, so a fact added later does not break
every implementation of [`Keys`](#keys). `now` is the framework's own clock
reading for the frame: a double-press window timed against another clock
closes on the wrong frame. `board` is passed with every press rather than fixed
when the keys are built, because the simulator can switch boards while it runs
and a decision made for the board that was there is wrong for the one that is.

## `Keypad`

The keys of a device, with a [`Keys`](#keys) reading them.

```text
pub struct Keypad
```

What [`Simulator::run`](simulator.md#simulatorrun) drives from the window's
key and click events, and what a test drives with no window. It keeps the
bookkeeping that is easy to get wrong: a key translated on the way down is
released as what it became, or that button stays held and auto-repeats for the
rest of the session.

The [`Keys` example](#keys) drives one.

### Creating a keypad

#### `Keypad::new`

A keypad reading meaning into presses through `keys`, nothing down.

```text
pub fn new(keys: Box<dyn Keys>) -> Self
```

### Pressing and releasing

#### `Keypad::down`

A key went down on `board`, delivered as whatever it turned out to mean.

```text
pub fn down(&mut self, backend: &Backend<PanelDisplay>, button: Button, now: u32, board: Board)
```

A second `down` for a key already down releases the first meaning before the
new one is decided.

#### `Keypad::up`

A key came up, released as whatever it was delivered as, if it was delivered.

```text
pub fn up(&mut self, backend: &Backend<PanelDisplay>, button: Button)
```

#### `Keypad::due`

Delivers anything that has come due, as a press and its release.

```text
pub fn due(&mut self, backend: &Backend<PanelDisplay>, now: u32)
```

A loop calls it through [`open_frame`](simulator.md#open_frame), which keeps it
ahead of the frame's events.

#### `Keypad::release_all`

Releases everything still held, on the backend it was pressed on.

```text
pub fn release_all(&mut self, backend: &Backend<PanelDisplay>)
```

For a board switch: the old board's backend, then [`Keys::reset`](#keysreset).

**See also:** [`Keys`](#keys), [`open_frame`](simulator.md#open_frame)

## `button_for`

What a key means.

```text
pub fn button_for(key: Keycode) -> Option<Button>
```

Named by meaning, never by position. The whole map is the key table in
[Keys and mouse](../running.md#keys-and-mouse), which `tests/keys.rs` checks
against this function row by row. **H** and **Q** are not in it: the loop
handles them before this is asked.

Escape is not `Back`, and cannot be. [`embedded-graphics-simulator`](https://crates.io/crates/embedded-graphics-simulator) turns it
into a quit event before a key press exists to map.

```rust
use xpui::Button;
use xpui_simulator::{Keycode, button_for};

assert_eq!(button_for(Keycode::Return), Some(Button::Confirm));
assert_eq!(button_for(Keycode::KpEnter), Some(Button::Confirm));
assert_eq!(button_for(Keycode::Backspace), Some(Button::Back));
assert_eq!(button_for(Keycode::Escape), None);
```

## `Control`

What a control key does to the window.

```text
pub enum Control
```

| Variant | |
|---|---|
| `Control::NextBoard` | Show the next board in the cycle the caller supplied, wrapping. |
| `Control::PreviousBoard` | Show the previous one. |
| `Control::ZoomIn` | One more window pixel per panel pixel. |
| `Control::ZoomOut` | One fewer, down to life size. |
| `Control::ToggleBody` | Show or hide the device body around the panel. |
| `Control::Screenshot` | Write the panel to a file. |

A `Button` is something the device has and the firmware reads; a control is the
window's own, and no device has a key that swaps its panel for another. Which
key sends which is the live-control table in
[Changing it while it runs](../running.md#changing-it-while-it-runs).
[`Session::apply`](simulator.md#sessionapply) applies one.

### Every control

#### `Control::ALL`

Every control, so the fixture test cannot check a subset of them and call the page correct.

```text
pub const ALL: [Control; 6] = [
    Control::NextBoard,
    Control::PreviousBoard,
    Control::ZoomIn,
    Control::ZoomOut,
    Control::ToggleBody,
    Control::Screenshot,
]
```

**See also:** [`control_for`](#control_for), [`Session`](simulator.md#session)

## `control_for`

What a key means, or `None` when it is not a control at all.

```text
pub fn control_for(key: Keycode, shift: bool) -> Option<Control>
```

| Parameter | Meaning |
|---|---|
| `key` | The key that went down. |
| `shift` | Whether either Shift was held. Only **B** reads it. |

Zoom answers to several keys each: `+` is Shift and `=` on most layouts and a
key of its own on a numeric pad. The loop asks this before
[`button_for`](#button_for), so a control key never arrives as a press.

```rust
use xpui_simulator::{Control, Keycode, control_for};

assert_eq!(control_for(Keycode::B, false), Some(Control::NextBoard));
assert_eq!(control_for(Keycode::B, true), Some(Control::PreviousBoard));
assert_eq!(control_for(Keycode::Equals, false), Some(Control::ZoomIn));
assert_eq!(control_for(Keycode::KpPlus, false), Some(Control::ZoomIn));
assert_eq!(control_for(Keycode::Return, false), None);
```

## `Touchscreen`

A touchscreen, and whatever is currently on it.

```text
pub struct Touchscreen
```

Driven by panel positions and millisecond timestamps, never by window pixels,
so every rule is a unit test. The thresholds are the firmware's: a tap is
reported where the finger went down, a swipe needs 60 px within 700 ms, a long
press needs 500 ms still. The table of all of them is in
[The mouse as a finger](../running.md#the-mouse-as-a-finger).

**A board with no touchscreen gets one that answers nothing.** A device that
cannot produce a coordinate must not have one invented for it.

**Example — a tap, a swipe, and a board with no touchscreen**

```rust
use xpui::{Point, SwipeDir};
use xpui_simulator::{Board, Touch, Touchscreen};

let mut screen = Touchscreen::for_board(Board::custom("touch reader", 480, 800, true));

screen.down(Point::new(240, 400), 0);
let lifted = screen.up(Some(Point::new(250, 410)), 120);
assert!(lifted.contains(Touch::Tap(Point::new(240, 400))), "where the finger landed");

screen.down(Point::new(240, 400), 1_000);
screen.moved(Point::new(240, 300));
let flicked = screen.up(Some(Point::new(240, 300)), 1_200);
assert!(flicked.contains(Touch::Swipe(SwipeDir::Up)));

let mut badge = Touchscreen::for_board(Board::custom("badge", 296, 128, false));
assert!(badge.down(Point::new(10, 10), 0).is_empty());
```

### Creating a touchscreen

#### `Touchscreen::for_board`

The touchscreen `board` has, or one that answers nothing if it has none.

```text
pub fn for_board(board: Board) -> Touchscreen
```

### A contact

#### `Touchscreen::is_down`

Whether a contact is in flight, so a caller can tell a drag from a mouse merely passing over the panel.

```text
pub fn is_down(&self) -> bool
```

#### `Touchscreen::down`

A finger landed.

```text
pub fn down(&mut self, at: Point, now_ms: u32) -> Touches
```

Reports [`Touch::Held`](#touch) at `at`.

#### `Touchscreen::moved`

The finger moved, still down.

```text
pub fn moved(&mut self, at: Point) -> Touches
```

Only for a position on the panel: a mouse that has left the panel reports
nothing, as a finger that has left the glass does.

#### `Touchscreen::tick`

A frame passed with the finger still down.

```text
pub fn tick(&mut self, now_ms: u32) -> Touches
```

Where a long press comes from, once per contact, because it is the one
classification with no event behind it.

#### `Touchscreen::up`

The finger lifted, at `at` if the release landed on the panel.

```text
pub fn up(&mut self, at: Option<Point>, now_ms: u32) -> Touches
```

Always reports [`Touch::Released`](#touch), then a tap, or a swipe with its
edge gesture when it began at an edge.

**See also:** [`Touch`](#touch), [`Touches`](#touches), [`route`](#route)

## `Touch`

What one raw event turned into.

```text
pub enum Touch
```

| Variant | |
|---|---|
| `Touch::Held` | A finger is down, at this pixel. |
| `Touch::LongPress` | Still down, still stationary, and now past the long-press interval — at the position it went *down*, for the same reason a tap is. |
| `Touch::Released` | The finger came up. |
| `Touch::Tap` | A completed tap, at the position the finger went **down**: the centroid drifts 10-20px as a finger rolls off during lift, so a tap routes to where the user touched, not where the finger let go. |
| `Touch::Swipe` | A flick: far enough, fast enough. |
| `Touch::Edge` | What that flick means, having started at an edge. |

What the window does with each:

| Touch | Reaches the framework as |
|---|---|
| Held, Released | the backend's touch down and touch up |
| Tap | a tap at that panel pixel |
| Swipe | a swipe, unless the same release also reported an edge gesture |
| Edge, Back | the backend's back gesture, and a press and release of `Back` |
| Edge, Home | the backend's home gesture, and nothing else: the app reads it there when it ticks, so it is never offered twice |
| LongPress | nothing of its own: the finger is still reported held on every frame, and the framework times a long press from that |
| Edge, Menu | nothing: classified, with no slot in the backend's input to go to |

An edge gesture is reported alongside its swipe, as the firmware reports it. A
consumer that honours the gesture ignores the swipe, which is how a reader
pages with a right swipe mid-screen while the same swipe from the left edge
goes back.

## `Touches`

The events one raw event produced, in the order they happened.

```text
pub struct Touches
```

At most three: a release, the swipe it completed, and that swipe's edge
gesture. A fixed array, so the frame loop does not allocate. It is also
`IntoIterator`.

### Reading them

#### `Touches::NONE`

Nothing happened: an event on a board with no touchscreen, or one with no contact in flight.

```text
pub const NONE: Touches
```

#### `Touches::iter`

The events, in the order they happened.

```text
pub fn iter(&self) -> impl Iterator<Item = Touch> + '_
```

#### `Touches::contains`

Whether `touch` is among them.

```text
pub fn contains(&self, touch: Touch) -> bool
```

#### `Touches::is_empty`

Whether nothing happened.

```text
pub fn is_empty(&self) -> bool
```

## `EdgeGesture`

A swipe that means more than its direction, because of where it began.

```text
pub enum EdgeGesture
```

| Variant | |
|---|---|
| `EdgeGesture::Back` | A left-to-right swipe that began in the left band. |
| `EdgeGesture::Home` | An upward swipe that began in the bottom band. |
| `EdgeGesture::Menu` | A downward swipe that began in the top band. |

The left band is 25% of the panel's width; the top and bottom bands are 14% of
its height. The swipe's own axis must dominate strictly, so a perfect diagonal
is no edge gesture. Anchoring them to an edge keeps mid-screen swipes free for
a screen that reads a plain `SwipeDir`.

## `Hit`

What a click turned out to be.

```text
pub enum Hit
```

| Variant | |
|---|---|
| `Hit::Panel` | On the panel, at this *panel* pixel — never a window pixel. |
| `Hit::Key` | On a physical key, which may mean a button or the home gesture. |
| `Hit::Body` | On the body, where there is nothing to press. |

Only `Panel` carries a coordinate, so a click off the panel cannot become a
touch: there is no position to give one.

## `route`

Where a click at `window_point` goes.

```text
pub fn route(layout: Option<&BezelLayout>, window_point: Point, panel_point: Option<Point>) -> Hit
```

| Parameter | Meaning |
|---|---|
| `layout` | The body on screen, or `None` when the board has none or it is hidden. |
| `window_point` | The click, in window pixels. |
| `panel_point` | What the window reported for it: `Some` panel pixel when it says the click was on the panel display. |

The panel pixel is the window's answer, not one worked out again here, but it
is believed only where [`BezelLayout::panel_holds`](panel.md#bezellayoutpanel_holds)
agrees. A click the window rounds onto the panel's edge from the shell beside it
is still the shell.

**Example — a click on the bezel**

```rust
use xpui::{Button, Point};
use xpui_boards_core::KeyAction;
use xpui_boards_xteink as xteink;
use xpui_simulator::{BezelLayout, Hit, Panel, route};

let board = xteink::X3;
let panel = Panel::of(board);
let bezel = board.bezel.expect("the X3 describes its body");
let layout = BezelLayout::new(bezel, panel.width, panel.height, panel.scale);

// The middle of the key labelled Prev, on the left edge.
let prev = bezel.buttons.iter().find(|key| key.label == "Prev").expect("a Prev key");
let on_prev = layout.to_window(prev.centre);
assert_eq!(route(Some(&layout), on_prev, None), Hit::Key(KeyAction::Press(Button::PageBack)));

// Bare shell just above the panel, whatever the window reports for it.
let corner = layout.panel_offset();
let above = Point::new(corner.x + 20, corner.y - 1);
assert_eq!(route(Some(&layout), above, Some(Point::new(20, 0))), Hit::Body);

// On the panel, the window's panel pixel is the answer.
let inside = Point::new(corner.x + 20, corner.y + 30);
assert_eq!(route(Some(&layout), inside, Some(Point::new(20, 30))), Hit::Panel(Point::new(20, 30)));
```

**See also:** [`Hit`](#hit), [`BezelLayout`](panel.md#bezellayout), [`Touchscreen`](#touchscreen)

## Re-exports

| | |
|---|---|
| `Keycode` | SDL2's key identifier, as `embedded-graphics-simulator` exports it: what [`button_for`](#button_for) and [`control_for`](#control_for) read. |
