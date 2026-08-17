//! The window, the frame loop, and the events that feed the backend.

use std::time::{Duration, Instant};

use embedded_graphics::geometry::{Point as WindowPoint, Size as PixelSize};
use embedded_graphics::pixelcolor::{BinaryColor, Rgb888};
use embedded_graphics_simulator::sdl2::{Keycode, MouseButton};
use embedded_graphics_simulator::{MultiWindow, OutputSettings, SimulatorDisplay, SimulatorEvent};

use xpui::screen::Screen;
use xpui::{App, Button, Point, SwipeDir};
use xpui_eg::{Backend, Palette};

use crate::bezel;
use crate::click::{Hit, Press, route};
use crate::keys::button_for;
use crate::layout::BezelLayout;
use crate::panel::{Panel, window_settings};

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
        // A press with no matching release would stay held forever, so both
        // the drag position and the key under the finger are tracked rather
        // than assumed.
        let mut press: Option<Press> = None;
        let mut held: Option<Button> = None;

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
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
                        let at = Point::new(point.x, point.y);
                        match route(layout.as_ref(), at, on_panel(backend, &window, point)) {
                            Hit::Panel(at_panel) => {
                                press = Some(Press {
                                    window: at,
                                    panel: at_panel,
                                });
                                backend.input(|state| state.touch_down(at_panel));
                            }
                            Hit::Button(button) => {
                                backend.press(button);
                                held = Some(button);
                                body_dirty = true;
                            }
                            Hit::Body => {}
                        }
                    }
                    SimulatorEvent::MouseMove { point } => {
                        // Only while a press that started on the panel is in
                        // flight: a drag off a physical button is not a touch.
                        if press.is_some()
                            && let Some(at) = on_panel(backend, &window, point)
                        {
                            backend.input(|state| state.touch_down(at));
                        }
                    }
                    SimulatorEvent::MouseButtonUp { point, mouse_btn } => {
                        if mouse_btn != MouseButton::Left {
                            continue;
                        }
                        // A key is released wherever the mouse came up, or it
                        // would stay held for the rest of the session.
                        if let Some(button) = held.take() {
                            backend.release(button);
                            body_dirty = true;
                        }
                        let Some(down) = press.take() else { continue };

                        backend.input(|state| state.touch_up());
                        if let Some(at) = down.tap(Point::new(point.x, point.y)) {
                            backend.tap(at);
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
