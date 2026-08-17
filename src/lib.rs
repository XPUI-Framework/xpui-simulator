//! Runs an [`xpui`] app in a desktop window.
//!
//! Not a separate rendering backend: it is
//! [`xpui-embedded-graphics`](xpui_eg) over `embedded-graphics-simulator`'s
//! display, plus a window, an event pump and a keyboard mapping. The pixels
//! are the same ones a device would get.
//!
//! `no_run` because it opens a window and pumps events until someone closes it:
//!
//! ```rust,no_run
//! # use xpui::{NavigationScreen, Screen, Text, View, vstack};
//! # use xpui_simulator::{Panel, Simulator};
//! # struct MainMenu;
//! # impl MainMenu { fn new() -> Self { MainMenu } }
//! # impl Screen for MainMenu {
//! #     type Message = ();
//! #     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("Menu")]) }
//! #     fn update(&mut self, _message: ()) {}
//! # }
//! fn main() {
//!     Simulator::new(Panel::DEFAULT).run(MainMenu::new());
//! }
//! ```
//!
//! # The device around the panel
//!
//! A [`Board`] that describes its body — see [`xpui_boards::Bezel`] — opens a
//! window larger than the panel, with the panel inset into it and the real
//! buttons drawn where a thumb would find them. Those buttons are clickable
//! and feed the same input the keyboard does. A board with no bezel opens a
//! window that is exactly the panel.
//!
//! # Keys
//!
//! | Key | Button |
//! |---|---|
//! | Up / Down | `Up` / `Down` |
//! | Left / Right | `Left` / `Right` |
//! | Enter or Space | `Confirm` |
//! | Backspace | `Back` |
//! | Page Up / Page Down | `PageBack` / `PageForward` |
//! | H | the home gesture |
//! | Q or Escape | quit |
//!
//! Escape cannot be `Back`: `embedded-graphics-simulator` turns it into
//! `SimulatorEvent::Quit` before [`button_for`] ever sees a key press.
//!
//! # The mouse as a finger
//!
//! A click and drag on the panel goes through [`Touchscreen`], which is
//! CrossPoint's touch model ported constant for constant: the same slops, the
//! same swipe window, the same long press, the same edge bands. A screen that
//! feels right in this window feels the same on the device, and a gesture the
//! firmware would refuse is refused here too. The scroll wheel is a plain
//! swipe as well, for boards with no touchscreen to classify for.
//!
//! A click anywhere but the panel is never a touch, and neither is a click on
//! a board that has no touchscreen: a device cannot receive one at a
//! coordinate it has no way of producing.

mod bezel;
mod click;
mod keys;
mod layout;
mod panel;
mod run;
mod touch;

pub use embedded_graphics_simulator::sdl2::Keycode;

pub use bezel::paint as paint_body;
pub use click::{Hit, route};
pub use keys::button_for;
pub use layout::BezelLayout;
pub use panel::{Panel, window_settings};
pub use run::Simulator;
pub use touch::{EdgeGesture, Touch, Touches, Touchscreen};
pub use xpui_boards::Board;

/// The crate's prose, compiled.
///
/// A README that does not build is worse than none: this crate's only usage
/// example passed the wrong form to its own macro for as long as nothing
/// tried it.
#[cfg(doctest)]
mod guides {
    #[doc = include_str!("../README.md")]
    pub mod readme {}
    /// `tests/keys.rs` also reads this file, but as *data* — it checks the key
    /// table against `button_for`. Reading is not compiling, and for a while
    /// that was mistaken for proof while both its Rust blocks went unbuilt.
    #[doc = include_str!("../docs/running.md")]
    pub mod running {}
}
