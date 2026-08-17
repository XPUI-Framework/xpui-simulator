//! What a finished travel means.
//!
//! The vocabulary a [`Touchscreen`](super::Touchscreen) reports in, and the
//! geometry that names one gesture rather than another: which way a flick
//! points, whether it was a flick at all, and what it means to have started at
//! an edge. Kept apart from the contact itself because this half is pure
//! arithmetic over two points and a duration — no state, nothing in flight.

use xpui::{Point, SwipeDir};

use super::{SWIPE_MAX_MS, SWIPE_MIN_PX};

/// How close to a side edge a swipe must *start* to mean more than its
/// direction, as a percentage of the panel. `EDGE_SWIPE_SIDE_FRAC` from the
/// SDK's `FreeInkUICore.h:231-236`.
///
/// The sides get the wider band because "a thumb reaching in from the bezel
/// lands further from the edge than a deliberate top/bottom pull".
const SIDE_BAND_PERCENT: i32 = 25;

/// The same, for the top and bottom edges. `EDGE_SWIPE_TOP_BOTTOM_FRAC`.
const TOP_BOTTOM_BAND_PERCENT: i32 = 14;

/// A swipe that means more than its direction, because of where it began.
///
/// Anchoring them to an edge is what keeps mid-screen swipes free for screens
/// that consume a plain [`SwipeDir`]: a reader paging with a left-to-right
/// swipe would otherwise navigate back instead of turning a page.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EdgeGesture {
    /// A left-to-right swipe that began in the left band.
    Back,
    /// An upward swipe that began in the bottom band.
    Home,
    /// A downward swipe that began in the top band.
    Menu,
}

/// What one raw event turned into.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Touch {
    /// A finger is down, at this pixel. Reported for every sample of the
    /// contact, which is the signal a drag needs.
    Held(Point),
    /// Still down, still stationary, and now past the long-press interval — at
    /// the position it went *down*, for the same reason a tap is.
    LongPress(Point),
    /// The finger came up. Reported whenever a contact ends, whatever else
    /// that contact turned out to be.
    Released,
    /// A completed tap, at the position the finger went **down**.
    ///
    /// `InputManager.cpp:567-570`: "the reported centroid drifts 10-20px as a
    /// finger rolls off during lift, which made small targets feel unreliable
    /// with release-point routing. A tap routes to where the user touched, not
    /// where the finger let go."
    Tap(Point),
    /// A flick: far enough, fast enough.
    Swipe(SwipeDir),
    /// What that flick means, having started at an edge. Reported *alongside*
    /// the swipe, as the firmware reports it — a consumer that honours the
    /// gesture is the one that has to ignore the swipe, which is how
    /// CrossPoint's reader keeps paging with a right swipe while the rest of
    /// the system navigates back with one.
    Edge(EdgeGesture),
}

/// How many events one raw event can produce.
///
/// A release is the busiest: the release itself, the swipe it completed, and
/// what that swipe means at its edge. A tap cannot join them — the release
/// slop and the swipe distance meet exactly, so a contact is one or the other.
const MAX_EVENTS: usize = 3;

/// The events one raw event produced, in the order they happened.
///
/// A fixed array rather than a `Vec`: this runs inside the frame loop, and
/// nothing in a frame loop should allocate.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Touches {
    events: [Option<Touch>; MAX_EVENTS],
}

impl Touches {
    /// Nothing happened: an event on a board with no touchscreen, or one with
    /// no contact in flight.
    pub const NONE: Touches = Touches {
        events: [None; MAX_EVENTS],
    };

    pub(super) fn of(touch: Touch) -> Touches {
        Touches::NONE.and(touch)
    }

    pub(super) fn and(mut self, touch: Touch) -> Touches {
        match self.events.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => *slot = Some(touch),
            // Unreachable while a tap and a swipe stay mutually exclusive.
            // This is the assertion that says so.
            None => debug_assert!(false, "more than {MAX_EVENTS} events: {touch:?}"),
        }
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = Touch> + '_ {
        self.events.iter().flatten().copied()
    }

    pub fn contains(&self, touch: Touch) -> bool {
        self.iter().any(|event| event == touch)
    }

    pub fn is_empty(&self) -> bool {
        self.events[0].is_none()
    }
}

impl IntoIterator for Touches {
    type Item = Touch;
    type IntoIter = core::iter::Flatten<core::array::IntoIter<Option<Touch>, MAX_EVENTS>>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter().flatten()
    }
}

/// Whether a travel of `(dx, dy)` over `held_ms` was a flick.
///
/// The distance is an **or**, not an and: `InputManager.cpp:700` rejects only a
/// gesture short on *both* axes, so 60 px along either one qualifies. A
/// mostly-horizontal swipe is not disqualified for having barely moved
/// vertically.
pub(super) fn swiped(dx: i32, dy: i32, held_ms: u32) -> bool {
    held_ms <= SWIPE_MAX_MS && (dx.abs() >= SWIPE_MIN_PX || dy.abs() >= SWIPE_MIN_PX)
}

/// Which way a travel of `(dx, dy)` points: the dominant axis, ties going
/// horizontal.
///
/// The SDK's `swipeDirection` (`FreeInkUICore.h:217-227`), whose `adx >= ady`
/// is what settles the tie.
pub(super) fn direction(dx: i32, dy: i32) -> SwipeDir {
    if dx.abs() >= dy.abs() {
        if dx < 0 {
            SwipeDir::Left
        } else {
            SwipeDir::Right
        }
    } else if dy < 0 {
        SwipeDir::Up
    } else {
        SwipeDir::Down
    }
}

/// What a swipe from `from` travelling `(dx, dy)` means on a panel of `size`,
/// if it started near enough to an edge to mean anything.
///
/// The SDK's `edgeSwipe` (`FreeInkUICore.h:238-267`). Each edge needs its own
/// axis **strictly** dominant, so a perfect diagonal is neither gesture — and
/// that is why [`direction`] cannot be reused here, since it breaks its ties
/// towards the horizontal.
pub(super) fn edge_gesture(size: (i32, i32), from: Point, dx: i32, dy: i32) -> Option<EdgeGesture> {
    let (width, height) = size;
    let horizontal = dx.abs() > dy.abs();
    let vertical = dy.abs() > dx.abs();

    let side = band(width, SIDE_BAND_PERCENT);
    let ends = band(height, TOP_BOTTOM_BAND_PERCENT);

    if horizontal && dx > 0 && from.x <= side {
        return Some(EdgeGesture::Back);
    }
    if vertical && dy < 0 && from.y >= height - ends {
        return Some(EdgeGesture::Home);
    }
    if vertical && dy > 0 && from.y <= ends {
        return Some(EdgeGesture::Menu);
    }
    None
}

/// How wide a band of `percent` is, on a panel `of` pixels across.
///
/// Truncating, as the firmware's `static_cast<int>(screenW * frac)` is, and
/// the comparisons against it are inclusive for the same reason.
fn band(of: i32, percent: i32) -> i32 {
    of * percent / 100
}
