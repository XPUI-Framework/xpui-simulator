//! Putting what the session holds onto the window.
//!
//! The loop decides *when*; this is *how*. Split out because the two are read
//! for different reasons — one to follow the order events are handled in, the
//! other to work out why something landed where it did on the glass.
//!
//! Order matters between two of these: the body covers the whole window, so
//! the panel goes on after it or the repaint erases the panel from the
//! framebuffer.

use embedded_graphics::geometry::Point as WindowPoint;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics_simulator::{MultiWindow, SimulatorDisplay};

use xpui_boards::KeyAction;
use xpui_eg::Backend;

use crate::panel::{PanelDisplay, window_settings};
use crate::press::Keypad;
use crate::session::Session;
use crate::{bezel, screenshot};

/// Opens a frame: the clock, the one-frame input, and anything held back.
///
/// A function rather than three lines in the loop, because a test that drives
/// a keypad has to open a frame the same way — and a hand-written copy of this
/// ordering is a copy that can drift from it without anything noticing.
///
/// **A press that has come due is delivered before any event is read.** It was
/// held back from an *earlier* frame, so it happened first; delivering it
/// after this frame's keys lets a press arriving one millisecond past the
/// window pre-empt the select it was too late to be part of, and that select
/// is then lost outright.
pub fn open_frame(keypad: &mut Keypad, backend: &Backend<PanelDisplay>, now: u32) {
    backend.begin_frame(now);
    keypad.due(backend, now);
}

/// Registers the panel where it now sits.
///
/// `add_display` is keyed by the display itself, so calling it again with the
/// same one moves it and calling it with a new one adds it. A switch needs
/// both, and a zoom needs the first: the scale lives in the output settings
/// rather than in anything the panel knows about itself.
pub(crate) fn bind_panel(window: &mut MultiWindow, session: &Session) {
    let offset = session.panel_offset();
    let output = window_settings(session.scale());
    session.backend().with_display(|display| {
        window.add_display(display, WindowPoint::new(offset.x, offset.y), &output)
    });
}

/// Repaints the device, or the bare desk when there is no body to draw.
///
/// Either way it covers the whole window, which is what erases the board
/// switched away from rather than leaving it showing round the edges of a
/// smaller one.
pub(crate) fn paint_body(
    window: &mut MultiWindow,
    body: &mut SimulatorDisplay<Rgb888>,
    session: &Session,
    held: Option<KeyAction>,
) {
    match session.layout() {
        Some(layout) => bezel::paint(body, &layout, held),
        None => bezel::backdrop(body),
    }
    window.update_display(body);
}

pub(crate) fn show_panel(window: &mut MultiWindow, session: &Session) {
    session
        .backend()
        .with_display(|display| window.update_display(display));
}

/// Writes the panel out and says where it went.
///
/// Printed rather than logged: a screenshot you cannot find is not a
/// screenshot, and this is a program whose whole output is a window.
pub(crate) fn announce(session: &Session) {
    let path = screenshot::capture(
        session.backend(),
        session.board().slug,
        &screenshot::directory(),
    );
    println!("wrote {}", path.display());
}
