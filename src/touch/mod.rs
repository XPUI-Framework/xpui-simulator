//! What a mouse becomes on a touchscreen.
//!
//! Classifying touch is the *simulator's* job and never the framework's:
//! `xpui` is handed taps, drags, swipes and gestures, and where those came
//! from is the host's business. A desktop has a mouse, so this is where a
//! mouse becomes a finger.
//!
//! The model is CrossPoint's, ported rule for rule and constant for constant
//! from the firmware rather than invented here. A simulator that classifies
//! differently from the device agrees with it right up to the point where it
//! matters — the 45-pixel finger roll that a window calls a tap and a reader
//! calls nothing at all.
//!
//! Nothing here opens a window or touches SDL. A [`Touchscreen`] is driven by
//! `(position, timestamp)` events, so every rule below is a unit test rather
//! than something to be checked by hand with a mouse and a stopwatch.
//!
//! Positions are *panel* pixels, which is the space a screen is laid out in
//! and the space the firmware measures in. A window's own pixels are a scaled
//! view of them and would put every threshold somewhere else at every zoom.
//!
//! This file is the contact: a finger's life from landing to lifting, and the
//! latches that decide what it was. [`gesture`] is what a finished travel
//! means.

mod gesture;

use xpui::Point;
use xpui_boards::Board;

pub use gesture::{EdgeGesture, Touch, Touches};

use gesture::{direction, edge_gesture, swiped};

// -- the firmware's numbers -----------------------------------------------
//
// All five are CrossPoint's, verbatim from the SDK's `InputManager.h:316-328`.

/// Motion past this cancels the *stationary* classifications — the hold and
/// the long press. `TOUCH_TAP_SLOP_PX`.
const TAP_SLOP_PX: i32 = 28;

/// The shortest travel that can be a swipe. `TOUCH_SWIPE_MIN_PX`.
const SWIPE_MIN_PX: i32 = 60;

/// How far a finger may have travelled and still have *tapped*, judged on
/// release. `TOUCH_TAP_RELEASE_SLOP_PX`, which the firmware defines as
/// `TOUCH_SWIPE_MIN_PX - 1` — deliberately not as `TOUCH_TAP_SLOP_PX`.
///
/// **The two slops differ on purpose, and it is the detail the whole model
/// hangs from.** `InputManager.cpp:561-566`: "Hold/long-press detection uses
/// the tighter 28 px stationary slop, but a released tap remains valid until
/// motion reaches the 60 px swipe threshold. Using the stationary threshold
/// here created a 29..59 px dead band where a normal finger roll was neither a
/// tap nor a swipe."
///
/// So a tap stays valid to 59 px and a swipe begins at 60. The two meet
/// exactly, with nothing in between, and `tests/touch.rs` fails if a gap is
/// ever reopened.
const TAP_RELEASE_SLOP_PX: i32 = SWIPE_MIN_PX - 1;

/// A swipe is a flick: it must cover the distance within this.
/// `TOUCH_SWIPE_MAX_MS`.
const SWIPE_MAX_MS: u32 = 700;

/// How long a stationary finger stays down before it is a long press.
/// `TOUCH_LONG_PRESS_MS` — shorter than the home key's, because a screen hold
/// has no button travel to absorb.
const LONG_PRESS_MS: u32 = 500;

/// One finger on the glass, from the moment it lands to the moment it lifts.
#[derive(Copy, Clone, Debug)]
struct Contact {
    /// Where it landed, and when. A tap, a long press and every edge band are
    /// judged from here.
    down: Point,
    down_ms: u32,
    /// The most recent sample. Only a drag and a swipe's end point use it.
    latest: Point,
    /// Some sample passed the stationary slop: no long press for this contact,
    /// however still the finger goes afterwards.
    ///
    /// Latched over the whole contact rather than recomputed from where the
    /// finger is now, because one that wandered out and came back was never
    /// holding still.
    left_tap_slop: bool,
    /// Some sample reached the swipe distance: this contact can no longer end
    /// in a tap.
    left_release_slop: bool,
    /// The long press is one-shot per contact.
    long_pressed: bool,
}

impl Contact {
    fn new(at: Point, now_ms: u32) -> Contact {
        Contact {
            down: at,
            down_ms: now_ms,
            latest: at,
            left_tap_slop: false,
            left_release_slop: false,
            long_pressed: false,
        }
    }

    /// Records another sample of the same contact.
    fn sample(&mut self, at: Point) {
        self.latest = at;
        let (dx, dy) = self.travel();
        // Strictly greater, as the firmware compares: 28 px of wobble is still
        // stationary, and 59 px is still a tap.
        self.left_tap_slop |= dx.abs() > TAP_SLOP_PX || dy.abs() > TAP_SLOP_PX;
        self.left_release_slop |= dx.abs() > TAP_RELEASE_SLOP_PX || dy.abs() > TAP_RELEASE_SLOP_PX;
    }

    /// How far the finger has come from where it landed.
    fn travel(&self) -> (i32, i32) {
        (self.latest.x - self.down.x, self.latest.y - self.down.y)
    }

    /// How long it has been down. Wrapping, because the host's clock is a
    /// millisecond `u32` and comes round every seven weeks.
    fn held_ms(&self, now_ms: u32) -> u32 {
        now_ms.wrapping_sub(self.down_ms)
    }
}

/// A touchscreen, and whatever is currently on it.
///
/// Built from a [`Board`], because only a board with a touchscreen has one: on
/// any other every method here answers nothing at all. A device that cannot
/// produce a coordinate must not have one invented for it, or a screen ships
/// depending on touch the hardware will never send.
#[derive(Copy, Clone, Debug)]
pub struct Touchscreen {
    /// The panel a gesture is measured against, in the pixels a screen is laid
    /// out in. The edge bands are fractions of this.
    size: (i32, i32),
    has_touch: bool,
    contact: Option<Contact>,
}

impl Touchscreen {
    pub fn for_board(board: Board) -> Touchscreen {
        Touchscreen {
            size: (board.width, board.height),
            has_touch: board.touch,
            contact: None,
        }
    }

    /// Whether a contact is in flight, so a caller can tell a drag from a
    /// mouse merely passing over the panel.
    pub fn is_down(&self) -> bool {
        self.contact.is_some()
    }

    /// A finger landed.
    pub fn down(&mut self, at: Point, now_ms: u32) -> Touches {
        if !self.has_touch {
            return Touches::NONE;
        }
        self.contact = Some(Contact::new(at, now_ms));
        Touches::of(Touch::Held(at))
    }

    /// The finger moved, still down.
    ///
    /// Only ever called with a position *on* the panel: a finger that has left
    /// the glass reports nothing, so neither does a mouse that has left the
    /// panel rectangle.
    pub fn moved(&mut self, at: Point) -> Touches {
        let Some(contact) = self.contact.as_mut() else {
            return Touches::NONE;
        };
        contact.sample(at);
        Touches::of(Touch::Held(at))
    }

    /// A frame passed with the finger still down.
    ///
    /// The long press is the one classification that fires mid-contact, so it
    /// hangs from a clock tick rather than from an event.
    pub fn tick(&mut self, now_ms: u32) -> Touches {
        let Some(contact) = self.contact.as_mut() else {
            return Touches::NONE;
        };
        if contact.long_pressed || contact.left_tap_slop || contact.held_ms(now_ms) < LONG_PRESS_MS
        {
            return Touches::NONE;
        }
        contact.long_pressed = true;
        Touches::of(Touch::LongPress(contact.down))
    }

    /// The finger lifted. `at` is `Some` when the release landed on the panel,
    /// in which case it counts as one last sample of the contact.
    pub fn up(&mut self, at: Option<Point>, now_ms: u32) -> Touches {
        let Some(mut contact) = self.contact.take() else {
            return Touches::NONE;
        };
        if let Some(at) = at {
            contact.sample(at);
        }

        let touches = Touches::of(Touch::Released);
        // A held finger that never travelled still ends in a tap, long press
        // or no long press. The firmware has an app ask for that tap to be
        // swallowed (`suppressTouchContact`) rather than deciding here.
        if !contact.left_release_slop {
            return touches.and(Touch::Tap(contact.down));
        }

        let (dx, dy) = contact.travel();
        if !swiped(dx, dy, contact.held_ms(now_ms)) {
            return touches;
        }
        let touches = touches.and(Touch::Swipe(direction(dx, dy)));
        match edge_gesture(self.size, contact.down, dx, dy) {
            Some(gesture) => touches.and(Touch::Edge(gesture)),
            None => touches,
        }
    }
}
