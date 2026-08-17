//! What a click on the window means.
//!
//! The panel is inset into a body, so a mouse position is in a space nothing
//! else uses: the window's. This is where it stops being one. A click either
//! reaches the firmware as a panel coordinate, presses a physical key, or lands
//! on bare shell and does nothing — and only the first of those three carries a
//! coordinate at all, which is what makes "a click off the panel is never a
//! touch" a property of the type rather than of a branch somebody has to
//! remember.

use xpui::{Button, Point};

use crate::layout::BezelLayout;

/// What a click turned out to be.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// On the panel, at this *panel* pixel — never a window pixel.
    Panel(Point),
    /// On a physical button.
    Button(Button),
    /// On the body, where there is nothing to press.
    Body,
}

/// Where a click at `window_point` goes.
///
/// `panel_point` is what the window reported for it: `Some` when the click
/// landed on the panel display, `None` when it did not. The translation is the
/// window's answer rather than one recomputed here — it already knows the
/// inset and the scale, and a second copy of that arithmetic is exactly the
/// drift this routing exists to avoid.
///
/// Only [`Hit::Panel`] carries a coordinate, and it is only ever the panel's.
/// That is the shape of the rule that a click outside the panel is never a
/// touch: there is no variant that could deliver a window pixel to a device
/// that has no such coordinate.
pub fn route(layout: Option<&BezelLayout>, window_point: Point, panel_point: Option<Point>) -> Hit {
    let on_body = layout.is_some_and(|layout| !layout.panel_holds(window_point));

    match (panel_point, on_body) {
        (Some(at), false) => Hit::Panel(at),
        _ => match layout.and_then(|layout| layout.button_at(window_point)) {
            Some(button) => Hit::Button(button),
            None => Hit::Body,
        },
    }
}

/// How far a release may drift from its press and still be a tap, in window
/// pixels.
///
/// Window rather than panel pixels because this is about the hand, not about
/// the screen: a shaky click is the same wobble whether the panel is drawn at
/// 1:1 or blown up to three.
const DRIFT: i32 = 8;

/// A press on the panel, still in flight.
///
/// Both spaces are kept because each answers a different question, and holding
/// only one of them is how a tap ends up reported at a window coordinate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Press {
    /// Where it went down in the window. Only the drift is measured here.
    pub window: Point,
    /// Where it went down on the panel. What a tap is reported at — a screen is
    /// laid out in panel pixels and has no other coordinate to be tapped at.
    pub panel: Point,
}

impl Press {
    /// The tap this press became, released at `window_point`, or `None` when it
    /// drifted far enough to have been a drag.
    ///
    /// A tap is reported where the finger went *down*, which is what the
    /// framework expects and what stops a slightly shaky click landing on a
    /// different control. A drag reports nothing extra: its held positions
    /// already went through frame by frame, and a tap on top of them would fire
    /// whatever the finger happened to finish over.
    pub fn tap(&self, window_point: Point) -> Option<Point> {
        let drifted = (self.window.x - window_point.x).abs() > DRIFT
            || (self.window.y - window_point.y).abs() > DRIFT;
        (!drifted).then_some(self.panel)
    }
}
