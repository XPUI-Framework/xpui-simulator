//! The window, the frame loop, and the events that feed the backend.

use std::time::{Duration, Instant};

use embedded_graphics::geometry::{Point as WindowPoint, Size as PixelSize};
use embedded_graphics::pixelcolor::{BinaryColor, Rgb888};
use embedded_graphics_simulator::sdl2::{Keycode, MouseButton};
use embedded_graphics_simulator::{MultiWindow, OutputSettings, SimulatorDisplay, SimulatorEvent};

use xpui::screen::Screen;
use xpui::{App, Button, Point, SwipeDir};
use xpui_boards::KeyAction;
use xpui_eg::{Backend, Palette};

use crate::bezel;
use crate::click::{Hit, route};
use crate::keys::button_for;
use crate::layout::BezelLayout;
use crate::panel::{Panel, window_settings};
use crate::touch::{EdgeGesture, Touch, Touches, Touchscreen};

/// The panel display, which is the only thing the firmware can draw on.
type PanelDisplay = SimulatorDisplay<BinaryColor>;

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
        let panel = self.panel;
        let display: PanelDisplay =
            SimulatorDisplay::new(PixelSize::new(panel.width as u32, panel.height as u32));

        // Sized from the board so the chrome matches the panel — the whole
        // reason the presets exist. A 296x128 strip laid out with 480x800
        // chrome shows no list rows at all.
        let backend = Backend::leak_for_board(display, panel.board, Palette::INK_IS_ON);
        // Safety: one window, one thread, and nothing has rendered yet.
        unsafe { xpui::host::install(backend) };

        let layout = panel
            .board
            .bezel
            .map(|bezel| BezelLayout::new(bezel, panel.width, panel.height, panel.scale));
        let inset = layout
            .as_ref()
            .map_or(Point::ORIGIN, BezelLayout::panel_offset);
        let (window_width, window_height) = panel.window_size();

        let mut window = MultiWindow::new(
            &self.title,
            PixelSize::new(window_width as u32, window_height as u32),
        );

        // Two displays, in paint order. The body covers the whole window and
        // goes down first; the panel lands on top at the inset, which is what
        // keeps everything the firmware draws inside the panel rectangle.
        let mut body: Option<SimulatorDisplay<Rgb888>> = layout.as_ref().map(|_| {
            SimulatorDisplay::new(PixelSize::new(window_width as u32, window_height as u32))
        });
        if let Some(body) = &body {
            window.add_display(body, WindowPoint::zero(), &OutputSettings::default());
        }
        let output = window_settings(panel.scale);
        backend.with_display(|display| {
            window.add_display(display, WindowPoint::new(inset.x, inset.y), &output)
        });

        let mut app = App::new(root);
        let started = Instant::now();
        // The mouse, as a finger. Built from the board, so a device with no
        // touchscreen classifies nothing at all.
        let mut touch = Touchscreen::for_board(panel.board);
        // A press with no matching release would stay held forever, so the key
        // under the finger is tracked rather than assumed.
        let mut held: Option<KeyAction> = None;

        // Paint once before the loop. `MultiWindow` creates its SDL window in
        // its constructor, so `events()` is safe here — but the first frame
        // would otherwise show an empty framebuffer until something changed.
        app.render();
        backend.clear_dirty();
        if let (Some(body), Some(layout)) = (body.as_mut(), layout.as_ref()) {
            bezel::paint(body, layout, held);
            window.update_display(body);
        }
        backend.with_display(|display| window.update_display(display));
        window.flush();

        // The body repaints on its own schedule. Tying it to the panel's dirty
        // flag would leave a held button unlit until the firmware happened to
        // draw something.
        let mut body_dirty = false;
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
            backend.begin_frame(now);

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
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
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
                                    KeyAction::Press(button) => backend.press(button),
                                    KeyAction::Home => app.home_gesture(),
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
                        if touch.is_down()
                            && let Some(at) = panel_point(backend, &window, layout.as_ref(), point)
                        {
                            deliver(backend, &mut app, touch.moved(at));
                        }
                    }
                    SimulatorEvent::MouseButtonUp { point, mouse_btn } => {
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
                        // A key is released wherever the mouse came up, or it
                        // would stay held for the rest of the session.
                        if let Some(action) = held.take() {
                            if let KeyAction::Press(button) = action {
                                backend.release(button);
                            }
                            body_dirty = true;
                        }
                        // A release off the panel ends the contact without
                        // being a sample of it: there is no panel pixel to say
                        // the finger lifted at.
                        let at = panel_point(backend, &window, layout.as_ref(), point);
                        deliver(backend, &mut app, touch.up(at, now));
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

            // The long press is the one classification with no event behind
            // it: the finger is still down and still where it landed, and what
            // has changed is only the clock.
            deliver(backend, &mut app, touch.tick(now));

            app.tick();
            let panel_dirty = app.render_if_dirty();
            if panel_dirty {
                backend.clear_dirty();
            }

            if !panel_dirty && !body_dirty {
                // Nothing changed, so nothing is repainted. SDL still has to
                // be pumped every frame or the OS decides the app has hung,
                // and without a pause the loop spins a core doing exactly that.
                std::thread::sleep(Duration::from_millis(8));
                window.flush();
                continue;
            }

            if body_dirty {
                if let (Some(body), Some(layout)) = (body.as_mut(), layout.as_ref()) {
                    bezel::paint(body, layout, held);
                    window.update_display(body);
                }
                body_dirty = false;
            }
            // Always after the body: the body covers the whole window, so
            // repainting it underneath erases the panel from the framebuffer.
            backend.with_display(|display| window.update_display(display));
            window.flush();
        }
    }
}

/// Where a mouse position is *on the panel*, or `None` when it is not there at
/// all.
///
/// Routed rather than believed. A position one pixel above the panel comes
/// back from the window as panel `(0, 0)`, because it divides by the pixel
/// pitch and truncates — and a stray `(0, 0)` sample in the middle of a
/// contact turns a tap into a swipe right across the screen.
fn panel_point(
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

/// Hands what the touchscreen decided to the backend, and to the app for the
/// one gesture the framework owns rather than reports.
fn deliver(backend: &Backend<PanelDisplay>, app: &mut App, touches: Touches) {
    // An edge swipe arrives as both the swipe and its edge meaning, as it does
    // on the device. Only the meaning is delivered: CrossPoint's
    // `ActivityManager` consumes the home gesture before any activity sees the
    // swipe (`ActivityManager.cpp:75-81`), and feeding both here would go home
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
                // device: `MappedInputManager.cpp:280-288` folds it into the
                // logical button, so an activity handling the key handles the
                // swipe without knowing there was one. Released in the same
                // frame, or it would auto-repeat.
                backend.press(Button::Back);
                backend.release(Button::Back);
            }
            Touch::Edge(EdgeGesture::Home) => {
                backend.input(|state| state.home_gesture());
                // The system gesture, handled above the screen stack — the
                // same route the H key takes.
                app.home_gesture();
            }
            // Neither has anywhere to go yet: `InputSource` has no menu
            // gesture and no long press, and CrossPoint's own FFI has no
            // `cpp_input_was_menu_gesture` either. They are classified rather
            // than dropped so that wiring one up is a change here and not a
            // second touch model.
            Touch::Edge(EdgeGesture::Menu) | Touch::LongPress(_) => {}
        }
    }
}

/// What the window says a raw mouse position is on the panel.
///
/// The window owns the inset and the scale and does the subtraction itself;
/// a second copy of that arithmetic here is the drift the routing exists to
/// avoid. `None` means the click was not on the panel at all.
fn on_panel(
    backend: &Backend<PanelDisplay>,
    window: &MultiWindow,
    at: WindowPoint,
) -> Option<Point> {
    backend
        .with_display(|display| window.translate_mouse_position(display, at))
        .map(|point| Point::new(point.x, point.y))
}
