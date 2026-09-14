//! What the window's mouse events mean to the backend.
//!
//! The window speaks in its own pixels and the framework speaks in the panel's.
//! Between them sits the routing, which decides whether a position is on the
//! panel at all, and the touchscreen, which decides what a contact *was*. This
//! is where what the two decided is handed to the backend, and to nothing
//! else: the framework reads every gesture out of the backend's input.

use embedded_graphics::geometry::Point as WindowPoint;
use embedded_graphics_simulator::MultiWindow;

use xpui::{Button, Point};
use xpui_eg::Backend;

use crate::click::{Hit, route};
use crate::layout::BezelLayout;
use crate::panel::PanelDisplay;
use crate::touch::{EdgeGesture, Touch, Touches};

/// Where a mouse position is *on the panel*, or `None` when it is not there at
/// all.
///
/// Routed rather than believed. A position one pixel above the panel comes
/// back from the window as panel `(0, 0)`, because it divides by the pixel
/// pitch and truncates — and a stray `(0, 0)` sample in the middle of a
/// contact turns a tap into a swipe right across the screen.
pub fn panel_point(
    backend: &Backend<PanelDisplay>,
    window: &MultiWindow,
    layout: Option<&BezelLayout>,
    at: WindowPoint,
) -> Option<Point> {
    match route(
        layout,
        Point::new(at.x, at.y),
        on_panel(backend, window, at),
    ) {
        Hit::Panel(point) => Some(point),
        Hit::Key(_) | Hit::Body => None,
    }
}

/// Hands what the touchscreen decided to the backend.
pub fn deliver(backend: &Backend<PanelDisplay>, touches: Touches) {
    // An edge swipe arrives as both the swipe and its edge meaning, as on the
    // device, and only the meaning is delivered: feeding both would go home
    // and move focus down in the same frame.
    let edged = touches.iter().any(|touch| matches!(touch, Touch::Edge(_)));

    for touch in touches {
        match touch {
            Touch::Held(at) => backend.input(|state| state.touch_down(at)),
            Touch::Released => backend.input(|state| state.touch_up()),
            Touch::Tap(at) => backend.tap(at),
            Touch::Swipe(direction) if !edged => backend.swipe(direction),
            Touch::Swipe(_) => {}
            Touch::Edge(EdgeGesture::Back) => {
                backend.input(|state| state.back_gesture());
                // And as a `Back` press, which is what the gesture *is* on the
                // device, so an activity handling the key handles the swipe.
                // Released in the same frame, or it would auto-repeat.
                backend.press(Button::Back);
                backend.release(Button::Back);
            }
            // Into the backend's input and nowhere else: the app reads the home
            // gesture from there when it ticks, so handing it to the app as
            // well would offer it twice.
            Touch::Edge(EdgeGesture::Home) => backend.input(|state| state.home_gesture()),
            // Nothing of its own to deliver. The finger is still reported held
            // on every frame, since `touch_down` lasts until `touch_up`, and a
            // long press is the framework's to time from that.
            Touch::LongPress(_) => {}
            // Nowhere to go: `InputState` has no menu gesture. Classified rather
            // than dropped, so wiring one up is a change here and not a second
            // touch model.
            Touch::Edge(EdgeGesture::Menu) => {}
        }
    }
}

/// What the window says a raw mouse position is on the panel.
///
/// The window owns the inset and the scale and does the subtraction itself.
/// `None` means the click was not on the panel at all.
pub fn on_panel(
    backend: &Backend<PanelDisplay>,
    window: &MultiWindow,
    at: WindowPoint,
) -> Option<Point> {
    backend
        .with_display(|display| window.translate_mouse_position(display, at))
        .map(|point| Point::new(point.x, point.y))
}
