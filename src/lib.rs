//! Runs an [`xpui`] app in a desktop window.
//!
//! Not a separate rendering backend: it is
//! [`xpui-embedded-graphics`](xpui_eg) over `embedded-graphics-simulator`'s
//! display, plus a window, an event pump and a keyboard mapping. The pixels
//! are the same ones a device would get.
//!
//! ```rust,ignore
//! fn main() {
//!     Simulator::new(Panel::PORTRAIT).run(MainMenu::new());
//! }
//! ```
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
//! A click is a tap, a drag reports held positions, and the scroll wheel is a
//! swipe — so touch behaviour can be exercised without a panel.

use std::time::Instant;

use embedded_graphics::pixelcolor::{BinaryColor, Rgb888};
pub use embedded_graphics_simulator::sdl2::Keycode;
use embedded_graphics_simulator::sdl2::MouseButton;
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettings, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent,
    Window,
};

use xpui::screen::Screen;
use xpui::{App, Button, Point, SwipeDir};
pub use xpui_boards::Board;
use xpui_eg::{Backend, Palette};

/// The panel to simulate.
#[derive(Copy, Clone, Debug)]
pub struct Panel {
    pub width: i32,
    pub height: i32,
    /// How many window pixels one panel pixel occupies. A 1-bit panel at 1:1
    /// is hard to read on a high-density display.
    pub scale: u32,
    /// The device being simulated, passed to the backend whole.
    ///
    /// Carried rather than rebuilt from `width`/`height`: a reconstructed board
    /// loses its name, its refresh time and whether it has a touchscreen, and
    /// a Badger 2040 simulated as a touch device is not the Badger 2040.
    pub board: Board,
}

impl Panel {
    /// The default when no board is named. A real device rather than a
    /// placeholder, so what opens is something that exists.
    pub const DEFAULT: Panel = Panel::of(Board::X4);

    /// The panel a [`Board`] has.
    ///
    /// The board is the shared description a firmware reads too, so a screen
    /// developed in this window and the same screen flashed to the hardware
    /// are laid out against identical numbers. Small panels are scaled up:
    /// a Badger 2040 at 1:1 is a 296x128 window, which is a postage stamp on a
    /// modern display.
    pub const fn of(board: Board) -> Panel {
        Panel {
            width: board.width,
            height: board.height,
            scale: Panel::scale_for(board.width, board.height),
            board,
        }
    }

    /// The largest whole scale whose window still fits a modest display.
    ///
    /// Both dimensions matter: keying off height alone would blow a 800x480
    /// panel up to 1600 pixels wide. The budget is deliberately conservative —
    /// a window that does not fit cannot be moved back on screen on every
    /// desktop, and zooming in is a keypress away.
    const fn scale_for(width: i32, height: i32) -> u32 {
        const MAX_WIDTH: i32 = 1200;
        const MAX_HEIGHT: i32 = 900;
        const MAX_SCALE: i32 = 3;

        let mut scale = MAX_SCALE;
        while scale > 1 {
            if width * scale <= MAX_WIDTH && height * scale <= MAX_HEIGHT {
                break;
            }
            scale -= 1;
        }
        scale as u32
    }

    pub fn scaled(mut self, scale: u32) -> Self {
        self.scale = scale;
        self
    }
}

/// A window, a backend, and the loop between them.
pub struct Simulator {
    panel: Panel,
    title: String,
    max_frames: Option<u32>,
}

impl Simulator {
    pub fn new(panel: Panel) -> Self {
        Simulator {
            panel,
            title: String::from("xpui"),
            max_frames: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Stops after `frames` frames instead of waiting for the window to close.
    ///
    /// This is what makes the simulator testable. Without it the only way out
    /// of the loop is a person clicking something, so a CI run — or a check
    /// that the loop even starts — hangs until something kills it.
    ///
    /// ```bash
    /// SDL_VIDEODRIVER=dummy cargo run -p xpui-gallery -- --frames 30
    /// ```
    pub fn frames(mut self, frames: u32) -> Self {
        self.max_frames = Some(frames);
        self
    }

    /// Runs `root` until the app finishes or the window closes.
    pub fn run<S: Screen + 'static>(self, root: S) {
        let display: SimulatorDisplay<BinaryColor> =
            SimulatorDisplay::new(embedded_graphics::geometry::Size::new(
                self.panel.width as u32,
                self.panel.height as u32,
            ));

        // Sized from the board so the chrome matches the panel — the whole
        // reason the presets exist. A 296x128 strip laid out with 480x800
        // chrome shows no list rows at all.
        let backend = Backend::leak_for_board(display, self.panel.board, Palette::INK_IS_ON);
        // Safety: one window, one thread, and nothing has rendered yet.
        unsafe { xpui::host::install(backend) };

        let output = window_settings(self.panel.scale);
        let mut window = Window::new(&self.title, &output);

        let mut app = App::new(root);
        let started = Instant::now();
        // A press with no matching release would stay held forever, so the
        // drag position is tracked rather than assumed.
        let mut dragging_from: Option<Point> = None;

        // Paint once before the loop. `Window::events()` panics outright if it
        // is called before `update()` has created the SDL window — which the
        // loop below does on its very first iteration. This is not a nicety;
        // without it the simulator dies on startup, every time.
        app.render();
        backend.clear_dirty();
        backend.with_display(|display| window.update(display));

        let mut frames: u32 = 0;

        'outer: while app.is_running() {
            if let Some(limit) = self.max_frames
                && frames >= limit
            {
                break;
            }
            frames += 1;
            backend.begin_frame(started.elapsed().as_millis() as u32);

            for event in window.events() {
                match event {
                    SimulatorEvent::Quit => break 'outer,
                    SimulatorEvent::KeyDown { keycode, .. } => {
                        if keycode == Keycode::Q {
                            break 'outer;
                        }
                        if keycode == Keycode::H {
                            app.home_gesture();
                            continue;
                        }
                        if let Some(button) = button_for(keycode) {
                            backend.press(button);
                        }
                    }
                    SimulatorEvent::KeyUp { keycode, .. } => {
                        if let Some(button) = button_for(keycode) {
                            backend.release(button);
                        }
                    }
                    SimulatorEvent::MouseButtonDown { point, mouse_btn } => {
                        if mouse_btn == MouseButton::Left {
                            let at = Point::new(point.x, point.y);
                            dragging_from = Some(at);
                            backend.input(|state| state.touch_down(at));
                        }
                    }
                    SimulatorEvent::MouseMove { point } => {
                        if dragging_from.is_some() {
                            backend.input(|state| state.touch_down(Point::new(point.x, point.y)));
                        }
                    }
                    SimulatorEvent::MouseButtonUp { point, mouse_btn } => {
                        if mouse_btn == MouseButton::Left {
                            // The tap is reported at where the finger went
                            // down, which is what the framework expects and
                            // what stops a slight drag reading as a tap
                            // somewhere else.
                            let down = dragging_from.take();
                            backend.input(|state| state.touch_up());
                            if let Some(from) = down {
                                let drifted =
                                    (from.x - point.x).abs() > 8 || (from.y - point.y).abs() > 8;
                                if !drifted {
                                    backend.tap(from);
                                }
                            }
                        }
                    }
                    SimulatorEvent::MouseWheel { scroll_delta, .. } => {
                        let direction = match scroll_delta.y.signum() {
                            1 => SwipeDir::Down,
                            -1 => SwipeDir::Up,
                            _ => SwipeDir::None,
                        };
                        if direction != SwipeDir::None {
                            backend.swipe(direction);
                        }
                    }
                }
            }

            app.tick();
            if app.render_if_dirty() {
                backend.clear_dirty();
                backend.with_display(|display| window.update(display));
            } else {
                // Nothing changed, so nothing is pushed. Without a pause the
                // loop spins a core doing exactly that.
                std::thread::sleep(std::time::Duration::from_millis(8));
                backend.with_display(|display| window.update(display));
            }
        }
    }
}

/// How the window presents one panel pixel.
///
/// Out of line so a test can read it back. Inline in the frame loop it was
/// unreachable, and the one line that matters here was wrong for as long as
/// nobody could assert on it.
pub fn window_settings(scale: u32) -> OutputSettings {
    OutputSettingsBuilder::new()
        .scale(scale)
        // Explicit, and load-bearing. `theme()` does
        // `pixel_spacing.get_or_insert(1)` as a side effect, which puts a gap
        // between every panel pixel: the window comes out roughly twice the
        // size it should be, and mouse coordinates arrive halved, because the
        // pitch used to unmap them is `scale + spacing`.
        .pixel_spacing(0)
        // Ink on paper rather than pixels on black: these are e-ink panels,
        // and a white-on-black preview reads as a bug. The default theme is an
        // identity map, so with ink as `On` it would render white.
        .theme(BinaryColorTheme::Custom {
            color_on: Rgb888::new(0x1A, 0x1A, 0x1A),
            color_off: Rgb888::new(0xF2, 0xF2, 0xEE),
        })
        .build()
}

/// The keyboard, as logical buttons.
///
/// Named by meaning, never by position: the framework's own contract is that a
/// screen asks for `Confirm` and the host decides what that is.
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

/// The crate's prose, compiled.
///
/// A README that does not build is worse than none: this crate's only usage
/// example passed the wrong form to its own macro for as long as nothing
/// tried it.
#[cfg(doctest)]
mod guides {
    #[doc = include_str!("../README.md")]
    pub mod readme {}
}
