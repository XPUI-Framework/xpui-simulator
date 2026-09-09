//! What a click on the window means.
//!
//! The panel is inset into a body, so a mouse position is in a space nothing
//! else uses: the window's. This is where it stops being one. A click either
//! reaches the firmware as a panel coordinate, presses a physical key, or lands
//! on bare shell and does nothing — and only the first of those three carries a
//! coordinate at all, which is what makes "a click off the panel is never a
//! touch" a property of the type rather than of a branch somebody has to
//! remember.

use xpui::Point;
use xpui_boards_core::KeyAction;

use crate::layout::BezelLayout;

/// What a click turned out to be.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// On the panel, at this *panel* pixel — never a window pixel.
    Panel(Point),
    /// On a physical key, which may mean a button or the home gesture.
    Key(KeyAction),
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
pub fn route(layout: Option<&BezelLayout>, window_point: Point, panel_point: Option<Point>) -> Hit {
    let on_body = layout.is_some_and(|layout| !layout.panel_holds(window_point));

    match (panel_point, on_body) {
        (Some(at), false) => Hit::Panel(at),
        _ => match layout.and_then(|layout| layout.button_at(window_point)) {
            Some(action) => Hit::Key(action),
            None => Hit::Body,
        },
    }
}
