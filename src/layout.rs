//! Where the device's millimetres land in the window.
//!
//! A [`Bezel`] describes the device in tenths of a millimetre. The window is in
//! pixels. Everything that converts between the two lives here, so the painter
//! and the event loop measure against one set of numbers rather than two.
//!
//! The panel is the anchor: it occupies exactly its own pixels times the scale,
//! and the body is drawn at whatever ratio that implies. Scaling the window
//! therefore moves nothing on the device — a button stays at the same
//! millimetre, which is what makes zoom cost no second set of numbers.

use xpui::Point;
use xpui_boards_core::{Bezel, KeyAction};

/// A device's body, its panel and its buttons, in window pixels.
#[derive(Copy, Clone, Debug)]
pub struct BezelLayout {
    bezel: Bezel,
    /// The panel's size in window pixels: its own pixels times the scale.
    panel_px: (i32, i32),
    /// Where the body's top-left corner sits in the window.
    ///
    /// Zero when the window is exactly the body. It is not, once the window is
    /// fixed and every board is letterboxed into the middle of it — and the
    /// offset has to live here rather than only in the painter, because the
    /// same numbers route every mouse position back onto the device.
    origin: Point,
}

impl BezelLayout {
    /// The layout for a bezel whose panel is `width` x `height` pixels, shown
    /// at `scale` window pixels each.
    ///
    /// `const` so [`Panel`](crate::Panel) can ask how big the window would be
    /// at a scale before committing to one. Without it the scale would have to
    /// be picked from the panel alone, which is how the Tufty 2040 — a small
    /// panel in a comparatively large body — first opened a window a third
    /// past what fits on a modest display.
    pub const fn new(bezel: Bezel, width: i32, height: i32, scale: u32) -> BezelLayout {
        BezelLayout {
            bezel,
            panel_px: (width * scale as i32, height * scale as i32),
            origin: Point::ORIGIN,
        }
    }

    /// The same body, moved to `origin` in the window.
    pub const fn at(mut self, origin: Point) -> BezelLayout {
        self.origin = origin;
        self
    }

    /// Where the body's top-left corner sits in the window.
    pub const fn origin(&self) -> Point {
        self.origin
    }

    /// The device this describes.
    pub const fn bezel(&self) -> &Bezel {
        &self.bezel
    }

    /// The body's own size in pixels, drawn at the panel's ratio.
    ///
    /// Not the window's size once the window is fixed and larger than this:
    /// it is how much of the window the device fills, which is what decides
    /// how much letterbox is left over.
    pub const fn window_size(&self) -> (i32, i32) {
        (self.px_x(self.bezel.body.0), self.px_y(self.bezel.body.1))
    }

    /// Where the panel's top-left corner sits in the window.
    ///
    /// The number handed to `MultiWindow::add_display`, and so the one it
    /// subtracts back off every mouse position.
    pub const fn panel_offset(&self) -> Point {
        self.to_window(self.bezel.panel_origin)
    }

    /// The panel's own size in window pixels.
    pub const fn panel_size(&self) -> (i32, i32) {
        self.panel_px
    }

    /// Whether a window pixel is on the panel.
    ///
    /// The authority on that question, rather than the window's own answer. A
    /// click a pixel above or left of the panel translates to `(0, 0)`: the
    /// window divides by the pixel pitch and truncates towards zero, which
    /// loses up to `scale - 1` pixels of the gap. Those pixels are body, and a
    /// device with no touchscreen must not receive a touch on them.
    pub const fn panel_holds(&self, at: Point) -> bool {
        let origin = self.panel_offset();
        at.x >= origin.x
            && at.y >= origin.y
            && at.x < origin.x + self.panel_px.0
            && at.y < origin.y + self.panel_px.1
    }

    /// Where a key's face lands, in window pixels.
    ///
    /// **One function, because two would drift.** The painter draws this rect
    /// and a test asserts against it; when each did its own arithmetic they
    /// disagreed by a pixel on four of the X3's keys — convert-then-halve is
    /// not halve-then-convert — and the committed golden recorded coordinates
    /// no code produced. The same argument `draw_option_popup` and
    /// `option_popup_row_rect` are one layout function for.
    pub const fn key_face(self, centre: (i32, i32), size: (i32, i32)) -> (Point, (i32, i32)) {
        let middle = self.to_window(centre);
        let (width, height) = self.to_window_size(size);
        (
            Point::new(middle.x - width / 2, middle.y - height / 2),
            (width, height),
        )
    }

    /// A point on the device, in window pixels.
    pub const fn to_window(self, at: (i32, i32)) -> Point {
        Point::new(
            self.origin.x + self.px_x(at.0),
            self.origin.y + self.px_y(at.1),
        )
    }

    /// A size on the device, in window pixels.
    pub const fn to_window_size(self, size: (i32, i32)) -> (i32, i32) {
        (self.px_x(size.0), self.px_y(size.1))
    }

    /// A window pixel, back in tenths of a millimetre.
    pub const fn to_device(self, at: Point) -> (i32, i32) {
        let x = at.x - self.origin.x;
        let y = at.y - self.origin.y;
        (
            divide(x * self.bezel.panel_size.0, self.panel_px.0),
            divide(y * self.bezel.panel_size.1, self.panel_px.1),
        )
    }

    /// What pressing the window pixel `at` would mean, if there is a key there.
    pub fn button_at(&self, at: Point) -> Option<KeyAction> {
        self.bezel
            .button_at(self.to_device(at))
            .map(|key| key.action)
    }

    /// Tenths of a millimetre across, in window pixels.
    ///
    /// Per axis, because the two ratios are not quite equal: the panel must
    /// occupy exactly its own pixels, so the body stretches to it rather than
    /// the other way round. The published sizes are close enough to square
    /// that the difference is under a percent.
    const fn px_x(&self, mm10: i32) -> i32 {
        divide(mm10 * self.panel_px.0, self.bezel.panel_size.0)
    }

    const fn px_y(&self, mm10: i32) -> i32 {
        divide(mm10 * self.panel_px.1, self.bezel.panel_size.1)
    }
}

/// Integer division that answers 0 rather than dividing by zero.
///
/// A bezel whose panel measures nothing is a data error, not a crash: the body
/// collapses to a point and the window is the panel, which is the same thing a
/// board with no bezel gets.
const fn divide(value: i32, by: i32) -> i32 {
    if by <= 0 { 0 } else { value / by }
}
