//! The keyboard, as logical buttons.

use embedded_graphics_simulator::sdl2::Keycode;
use xpui::Button;

/// What a key means.
///
/// Named by meaning, never by position: the framework's own contract is that a
/// screen asks for `Confirm` and the host decides what that is.
///
/// Escape is absent on purpose. `embedded-graphics-simulator` turns it into
/// `SimulatorEvent::Quit` upstream, so no key press ever arrives to be mapped,
/// and calling it `Back` would only make the documentation lie.
pub fn button_for(key: Keycode) -> Option<Button> {
    Some(match key {
        Keycode::Up => Button::Up,
        Keycode::Down => Button::Down,
        Keycode::Left => Button::Left,
        Keycode::Right => Button::Right,
        Keycode::Return | Keycode::KpEnter | Keycode::Space => Button::Confirm,
        Keycode::Backspace => Button::Back,
        Keycode::PageUp => Button::PageBack,
        Keycode::PageDown => Button::PageForward,
        _ => return None,
    })
}
