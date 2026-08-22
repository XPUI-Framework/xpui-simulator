//! The window, the frame loop, and the events that feed the backend.

use std::time::{Duration, Instant};

use embedded_graphics::geometry::{Point as WindowPoint, Size as PixelSize};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics_simulator::sdl2::{Keycode, Mod, MouseButton};
use embedded_graphics_simulator::{MultiWindow, OutputSettings, SimulatorDisplay, SimulatorEvent};

use xpui::screen::Screen;
use xpui::{App, Point, SwipeDir};
use xpui_boards::{Board, KeyAction};

use crate::click::{Hit, route};
use crate::controls::{Control, control_for};
use crate::feed::{deliver, on_panel, panel_point};
use crate::keys::button_for;
use crate::panel::Panel;
use crate::present::{announce, bind_panel, open_frame, paint_body, show_panel};
use crate::press::{Keypad, Keys, Raw};
use crate::session::Session;
use crate::touch::Touchscreen;

/// A window, a backend, and the loop between them.
pub struct Simulator {
    panel: Panel,
    boards: Vec<Board>,
    title: String,
    max_frames: Option<u32>,
    keys: Box<dyn Keys>,
}

impl Simulator {
    pub fn new(panel: Panel) -> Self {
        Simulator {
            panel,
            boards: vec![panel.board],
            title: String::from("xpui"),
            max_frames: None,
            keys: Box::new(Raw),
        }
    }

    /// The boards the board keys cycle through, in order.
    ///
    /// Left unset, the cycle is the one board the [`Panel`] was opened on.
    /// See [`Session::cycling`] for why that is the default and what happens
    /// to a list that leaves the panel's own board out.
    pub fn boards(mut self, boards: &[Board]) -> Self {
        self.boards = boards.to_vec();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Reads meaning into presses before the framework sees them.
    ///
    /// Left alone, presses arrive exactly as the hardware sent them. A board
    /// with fewer keys than it has meanings needs to fold two together, and
    /// that decision belongs to the firmware — see [`Keys`].
    pub fn keys(mut self, keys: impl Keys + 'static) -> Self {
        self.keys = Box::new(keys);
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

    /// The [`Session`] [`run`](Simulator::run) will drive.
    ///
    /// Public because `run` opens a window and pumps SDL until someone closes
    /// it, so nothing about it can be asserted on a CI machine. This is the
    /// part that can: everything the builder was told, resolved into the thing
    /// that answers key presses.
    pub fn session(&self) -> Session {
        Session::cycling(self.panel, &self.boards)
    }

    /// Runs `root` until the app finishes or the window closes.
    pub fn run<S: Screen + 'static>(self, root: S) {
        let mut session = self.session();
        let (window_width, window_height) = session.window_size();

        let mut window = MultiWindow::new(
            &self.title,
            PixelSize::new(window_width as u32, window_height as u32),
        );

        // Two displays, in paint order. The body covers the whole window and
        // goes down first; the panel lands on top at its offset, which is what
        // keeps everything the firmware draws inside the panel rectangle.
        //
        // One body display for every board, sized to the window rather than to
        // any one device. Its own painter places the shell inside it, so a
        // switch changes what is drawn and not what is registered — and the
        // window's display table cannot grow with the number of switches.
        let mut body: SimulatorDisplay<Rgb888> =
            SimulatorDisplay::new(PixelSize::new(window_width as u32, window_height as u32));
        window.add_display(&body, WindowPoint::zero(), &OutputSettings::default());
        bind_panel(&mut window, &session);

        let mut app = App::new(root);
        let started = Instant::now();
        // The mouse, as a finger. Built from the board, so a device with no
        // touchscreen classifies nothing at all.
        let mut touch = Touchscreen::for_board(session.board());
        // A press with no matching release would stay held forever, so the key
        // under the finger is tracked rather than assumed. This one is what the
        // body draws lit; what the framework receives is `presses`.
        let mut held: Option<KeyAction> = None;
        let mut keypad = Keypad::new(self.keys);

        // Paint once before the loop. `MultiWindow` creates its SDL window in
        // its constructor, so `events()` is safe here — but the first frame
        // would otherwise show an empty framebuffer until something changed.
        app.render();
        session.backend().clear_dirty();
        paint_body(&mut window, &mut body, &session, held);
        show_panel(&mut window, &session);
        window.flush();

        // The body repaints on its own schedule. Tying it to the panel's dirty
        // flag would leave a held button unlit until the firmware happened to
        // draw something.
        let mut body_dirty = false;
        // A screenshot asked for this frame, taken once it has painted.
        let mut shot = false;
        let mut frames: u32 = 0;

        'outer: while app.is_running() {
            if let Some(limit) = self.max_frames
                && frames >= limit
            {
                break;
            }
            frames += 1;
            // One clock reading for the whole frame, and the one the framework
            // is given. A long press timed against a different stamp from the
            // one a screen reads is a long press that fires on the wrong frame.
            let now = started.elapsed().as_millis() as u32;
            open_frame(&mut keypad, session.backend(), now);

            // Drained into a list first. The iterator borrows the window, and
            // a board switch has to re-register the panel display *before* the
            // next mouse event is routed — `translate_mouse_position` panics
            // for a display that was never added, so the switch cannot wait
            // until the batch is over.
            let events: Vec<SimulatorEvent> = window.events().collect();
            for event in events {
                match event {
                    SimulatorEvent::Quit => break 'outer,
                    SimulatorEvent::KeyDown {
                        keycode,
                        keymod,
                        repeat,
                    } => {
                        if keycode == Keycode::Q {
                            break 'outer;
                        }
                        // Auto-repeat is dropped for the controls only: held
                        // down, B would run through every board in a second
                        // and S would fill a directory.
                        if !repeat && let Some(control) = control_for(keycode, shifted(keymod)) {
                            if control == Control::Screenshot {
                                // Deferred to the end of the frame, once it has
                                // painted. Taken here, a press arriving in the
                                // same batch as anything that changed the
                                // screen would capture the frame before it —
                                // and one in the same batch as a switch would
                                // capture a panel nothing had drawn on yet.
                                shot = true;
                                continue;
                            }
                            let (leaving, previous) = (session.board(), session.backend());
                            if !session.apply(control) {
                                continue;
                            }
                            if session.board() != leaving {
                                // A key under the mouse is on a device that is
                                // no longer here, and so is the backend it went
                                // down on: released there rather than on a panel
                                // that never saw it pressed.
                                held = None;
                                keypad.release_all(previous);
                                touch = Touchscreen::for_board(session.board());
                                // The new backend has not started this frame.
                                open_frame(&mut keypad, session.backend(), now);
                            }
                            bind_panel(&mut window, &session);
                            app.invalidate();
                            body_dirty = true;
                            continue;
                        }
                        if keycode == Keycode::H {
                            app.home_gesture();
                            continue;
                        }
                        // Auto-repeat is dropped here too, and for a stronger
                        // reason than the controls': a key held on hardware
                        // sends one press and stays down, and the runtime does
                        // its own repeat off that. Letting SDL's repeat through
                        // re-arms that timer on every one of them, so a held
                        // key would step at the window manager's rate rather
                        // than the framework's — and anything reading two
                        // presses as a chord would see one from a key nobody
                        // pressed twice.
                        if repeat {
                            continue;
                        }
                        if let Some(button) = button_for(keycode) {
                            keypad.down(session.backend(), button, now, session.board());
                        }
                    }
                    SimulatorEvent::KeyUp { keycode, .. } => {
                        if let Some(button) = button_for(keycode) {
                            keypad.up(session.backend(), button);
                        }
                    }
                    SimulatorEvent::MouseButtonDown { point, mouse_btn } => {
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
                        let backend = session.backend();
                        let layout = session.layout();
                        let at = Point::new(point.x, point.y);
                        match route(layout.as_ref(), at, on_panel(backend, &window, point)) {
                            Hit::Panel(at_panel) => {
                                deliver(backend, &mut app, touch.down(at_panel, now));
                            }
                            Hit::Key(action) => {
                                // Home is a gesture rather than a press: the
                                // reader that has this key reports it from the
                                // touch controller, not from a pin.
                                match action {
                                    KeyAction::Press(button) => {
                                        keypad.down(backend, button, now, session.board())
                                    }
                                    KeyAction::Home => app.home_gesture(),
                                    // Drawn and pressable because the board has it;
                                    // it just does nothing yet.
                                    KeyAction::Unassigned => {}
                                }
                                held = Some(action);
                                body_dirty = true;
                            }
                            Hit::Body => {}
                        }
                    }
                    SimulatorEvent::MouseMove { point } => {
                        // Only while a contact that started on the panel is in
                        // flight, and only over the panel: a drag off a
                        // physical button is not a touch, and a finger that has
                        // left the glass reports nothing.
                        let backend = session.backend();
                        if touch.is_down()
                            && let Some(at) =
                                panel_point(backend, &window, session.layout().as_ref(), point)
                        {
                            deliver(backend, &mut app, touch.moved(at));
                        }
                    }
                    SimulatorEvent::MouseButtonUp { point, mouse_btn } => {
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
                        let backend = session.backend();
                        // A key is released wherever the mouse came up, or it
                        // would stay held for the rest of the session.
                        if let Some(action) = held.take() {
                            if let KeyAction::Press(button) = action {
                                keypad.up(backend, button);
                            }
                            body_dirty = true;
                        }
                        // A release off the panel ends the contact without
                        // being a sample of it: there is no panel pixel to say
                        // the finger lifted at.
                        let at = panel_point(backend, &window, session.layout().as_ref(), point);
                        deliver(backend, &mut app, touch.up(at, now));
                    }
                    SimulatorEvent::MouseWheel { scroll_delta, .. } => {
                        let direction = match scroll_delta.y.signum() {
                            1 => SwipeDir::Down,
                            -1 => SwipeDir::Up,
                            _ => SwipeDir::None,
                        };
                        if direction != SwipeDir::None {
                            session.backend().swipe(direction);
                        }
                    }
                }
            }

            // The long press is the one classification with no event behind
            // it: the finger is still down and still where it landed, and what
            // has changed is only the clock.
            deliver(session.backend(), &mut app, touch.tick(now));

            app.tick();
            let panel_dirty = app.render_if_dirty();
            if panel_dirty {
                session.backend().clear_dirty();
            }

            if !panel_dirty && !body_dirty {
                // Nothing changed, so nothing is repainted. SDL still has to
                // be pumped every frame or the OS decides the app has hung,
                // and without a pause the loop spins a core doing exactly that.
                std::thread::sleep(Duration::from_millis(8));
                window.flush();
            } else {
                if body_dirty {
                    paint_body(&mut window, &mut body, &session, held);
                    body_dirty = false;
                }
                // Always after the body: the body covers the whole window, so
                // repainting it underneath erases the panel from the
                // framebuffer.
                show_panel(&mut window, &session);
                window.flush();
            }

            if shot {
                announce(&session);
                shot = false;
            }
        }
    }
}

/// Whether either shift key was down.
fn shifted(keymod: Mod) -> bool {
    keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xpui_boards::Board;

    /// The builder keeps what it was handed.
    ///
    /// A unit test rather than one in `tests/`, because it reads a private
    /// field. Without it `boards` can drop its argument on the floor and every
    /// integration test still passes: they all build a [`Session`] directly.
    #[test]
    fn the_builder_keeps_the_list_it_was_given() {
        let mine = Board::custom("mine", 400, 300, false);
        let simulator = Simulator::new(Panel::of(mine));
        assert_eq!(
            simulator.boards,
            vec![mine],
            "unset, the cycle is the panel's own board"
        );

        // Described here rather than taken from `xpui-boards`' presets: this
        // crate names no device, and a list of three panels proves the order
        // is kept whatever is on them.
        let three = [
            Board::custom("first", 400, 300, false),
            Board::custom("second", 296, 128, false),
            Board::custom("third", 480, 800, true),
        ];
        let simulator = Simulator::new(Panel::of(three[0])).boards(&three);
        assert_eq!(simulator.boards, three, "and it is the caller's, in order");
    }
}
