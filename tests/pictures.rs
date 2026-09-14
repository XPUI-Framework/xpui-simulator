//! The pictures on the reference pages, drawn the way the window draws them.
//!
//! The window composes two displays: the body, painted over the whole window,
//! and the panel on top of it at its offset through `window_settings`. This
//! does the same into an image instead of onto SDL, so the picture of a device
//! on `docs/reference/panel.md` is what the window shows, not a mock-up of it.
//!
//! Each picture is a golden: a run compares what it draws against the committed
//! PNG byte for byte, and `UPDATE_SNAPSHOTS=1` rewrites it.

use std::fs;
use std::path::Path;

use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics_simulator::{OutputSettings, SimulatorDisplay};
use xpui::{App, List, ListRow, NavigationScreen, Screen, View};
use xpui_boards_xteink as xteink;
use xpui_simulator::{Panel, Session, paint_body, window_settings};

/// A settings screen, so the panel in the picture has something on it.
struct Settings;

impl Screen for Settings {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Frontlight").value("On").on_tap(()))
                .push(ListRow::new("Sleep after").value("5 min").on_tap(()))
                .push(ListRow::new("Free heap").value("182 KB")),
        )
    }

    fn update(&mut self, _message: ()) {}

    fn title(&self) -> Option<&'static str> {
        Some("Settings")
    }
}

#[test]
fn the_x3_in_its_body() {
    let session = Session::new(Panel::of(xteink::X3));
    App::new(Settings).render();

    let layout = session.layout().expect("the X3 describes its body");
    let (width, height) = session.window_size();
    let mut body: SimulatorDisplay<Rgb888> =
        SimulatorDisplay::new(Size::new(width as u32, height as u32));
    paint_body(&mut body, &layout, None);

    let mut image = body.to_rgb_output_image(&OutputSettings::default());
    let offset = session.panel_offset();
    session.backend().with_display(|panel| {
        image.draw_display(
            panel,
            Point::new(offset.x, offset.y),
            &window_settings(session.scale()),
        )
    });

    let drawn = Path::new(env!("CARGO_TARGET_TMPDIR")).join("panel_x3.png");
    image.save_png(&drawn).expect("the picture was written");
    let golden =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/screenshots/reference/panel_x3.png");

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() || !golden.exists() {
        fs::create_dir_all(golden.parent().expect("a parent")).expect("the golden's directory");
        fs::copy(&drawn, &golden).expect("the golden was written");
        return;
    }
    assert!(
        fs::read(&drawn).expect("the drawn picture") == fs::read(&golden).expect("the golden"),
        "{} no longer matches what the window draws; the new one is {}. \
         If the change is intended, re-run with UPDATE_SNAPSHOTS=1 and look at it.",
        golden.display(),
        drawn.display()
    );
}
