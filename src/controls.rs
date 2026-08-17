//! The keys that change the simulator itself rather than driving the screen.
//!
//! Separate from [`button_for`](crate::button_for) because they answer to a
//! different owner. A `Button` is something the device has and the firmware
//! reads; these are the window's own controls, and a device has no key that
//! swaps its own panel for a different one.
//!
//! They are checked *before* the button map, so nothing here can also arrive
//! as a press. None of these letters is a button today, and this ordering is
//! what keeps that true if one ever becomes one.

use embedded_graphics_simulator::sdl2::Keycode;

/// What a control key does to the window.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Control {
    /// Show the next board in `Board::ALL`, wrapping.
    NextBoard,
    /// Show the previous one.
    PreviousBoard,
    /// One more window pixel per panel pixel.
    ZoomIn,
    /// One fewer, down to life size.
    ZoomOut,
    /// Show or hide the device body around the panel.
    ToggleBody,
    /// Write the panel to a file.
    Screenshot,
}

impl Control {
    /// Every control, so the fixture test cannot check a subset of them and
    /// call the page correct.
    pub const ALL: [Control; 6] = [
        Control::NextBoard,
        Control::PreviousBoard,
        Control::ZoomIn,
        Control::ZoomOut,
        Control::ToggleBody,
        Control::Screenshot,
    ];
}

/// What a key means, or `None` when it is not a control at all.
///
/// `shift` distinguishes the only pair that needs it. Zoom accepts three keys
/// each because `+` is Shift and `=` on most layouts and its own key on a
/// numeric pad: a table that recognised only one of them would work on the
/// machine it was written on.
pub fn control_for(key: Keycode, shift: bool) -> Option<Control> {
    Some(match key {
        Keycode::B if shift => Control::PreviousBoard,
        Keycode::B => Control::NextBoard,
        Keycode::Plus | Keycode::Equals | Keycode::KpPlus => Control::ZoomIn,
        Keycode::Minus | Keycode::KpMinus => Control::ZoomOut,
        Keycode::E => Control::ToggleBody,
        Keycode::S => Control::Screenshot,
        _ => return None,
    })
}
