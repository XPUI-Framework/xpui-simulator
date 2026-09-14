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
//! # use xpui_simulator::Board;
//! # use xpui_simulator::{Panel, Simulator};
//! # struct MainMenu;
//! # impl MainMenu { fn new() -> Self { MainMenu } }
//! # impl Screen for MainMenu {
//! #     type Message = ();
//! #     fn body(&self) -> impl View<()> { NavigationScreen::new(vstack![0; Text::new("Menu")]) }
//! #     fn update(&mut self, _message: ()) {}
//! # }
//! fn main() {
//!     // Whichever device you are developing for. This crate has no default
//!     // one and no list of its own: it opens whatever board it is handed.
//!     // `xpui-boards-pimoroni`, `-xteink` and `-seeed` carry ready-made
//!     // ones — a crate per vendor, so you take the devices you target —
//!     // and `Board::custom` describes anything else.
//!     let mine = Board::custom("my reader", 480, 800, false);
//!     Simulator::new(Panel::of(mine)).run(MainMenu::new());
//! }
//! ```
//!
//! # The device around the panel
//!
//! A [`Board`] that describes its body — see [`xpui_boards_core::Bezel`] — opens a
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
//! # Changing it while it runs
//!
//! | Key | |
//! |---|---|
//! | B / Shift+B | the next board in the caller's cycle, or the previous one |
//! | + / - | zoom in and out |
//! | E | show or hide the device body |
//! | S | write the panel to `target/screenshots/` |
//!
//! **`B` walks the cycle its caller supplied**, and the example above supplied
//! one board, so it does nothing there. [`Simulator::boards`] is where an
//! application offers more; `xpui-gallery`'s offers eight.
//!
//! The screen stack survives a board switch: the same screen you had navigated
//! to is re-measured against the new panel and painted on it, which is the
//! whole point — every panel without a run each, and without navigating back
//! to the screen you wanted to look at each time.
//!
//! The window itself never changes size. It is opened once, large enough for
//! the largest board any of these keys can reach, and every smaller one is
//! letterboxed into the middle of it. That is also what limits zoom: a scale
//! whose device would not fit the window is refused, so a reader-sized panel
//! stays at life size and the small strips are where zooming actually buys
//! something. See [`Control`] and [`Session`].
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
//!
//! # Raw presses
//!
//! Keys arrive as the hardware sends them: one press per press, no repeat, and
//! no reading between them. A board with fewer keys than jobs has to fold one
//! into another — but *what two presses mean* is the firmware's decision, not
//! the simulator's, so
//! [`Simulator::keys`] hands them over before the framework sees them. See
//! [`Keys`].

#![deny(missing_docs)]

mod bezel;
mod click;
mod controls;
mod feed;
mod keys;
mod layout;
mod panel;
mod present;
mod press;
mod run;
mod screenshot;
mod session;
mod touch;

pub use embedded_graphics_simulator::sdl2::Keycode;

pub use bezel::paint as paint_body;
pub use click::{Hit, route};
pub use controls::{Control, control_for};
pub use keys::button_for;
pub use layout::BezelLayout;
pub use panel::{Panel, PanelDisplay, window_settings};
pub use present::open_frame;
pub use press::{Keypad, Keys, Press, Raw};
pub use run::Simulator;
pub use screenshot::capture as capture_panel;
pub use session::Session;
pub use touch::{EdgeGesture, Touch, Touches, Touchscreen};
pub use xpui_boards_core::Board;

/// The crate's prose, compiled: a page that does not build is worse than
/// none.
#[cfg(doctest)]
mod guides {
    #[doc = include_str!("../README.md")]
    pub mod readme {}
    /// `tests/keys.rs` also reads this file, as *data*: it checks the key
    /// table against `button_for`. Reading is not compiling.
    #[doc = include_str!("../docs/running.md")]
    pub mod running {}
    #[doc = include_str!("../docs/design.md")]
    pub mod design {}
    #[doc = include_str!("../docs/reference.md")]
    pub mod reference {}
    #[doc = include_str!("../docs/reference/simulator.md")]
    pub mod reference_simulator {}
    #[doc = include_str!("../docs/reference/panel.md")]
    pub mod reference_panel {}
    #[doc = include_str!("../docs/reference/input.md")]
    pub mod reference_input {}
}
