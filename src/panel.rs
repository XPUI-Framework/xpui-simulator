//! The panel to simulate, and how a window presents one of its pixels.

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics_simulator::{BinaryColorTheme, OutputSettings, OutputSettingsBuilder};

use xpui_boards::Board;

use crate::layout::BezelLayout;

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
            scale: Panel::scale_for(board),
            board,
        }
    }

    /// The largest whole scale whose window still fits a modest display.
    ///
    /// The *window*, which is the body when the board has described one and the
    /// panel when it has not. Both dimensions matter: keying off height alone
    /// would blow an 800x480 panel up to 1600 pixels wide, and keying off the
    /// panel alone opened a Tufty 2040 — a small panel in a comparatively large
    /// body — a third past the budget on both axes.
    ///
    /// The budget is deliberately conservative: a window that does not fit
    /// cannot be moved back on screen on every desktop.
    const fn scale_for(board: Board) -> u32 {
        const MAX_WIDTH: i32 = 1200;
        const MAX_HEIGHT: i32 = 900;
        const MAX_SCALE: u32 = 3;

        let mut scale = MAX_SCALE;
        while scale > 1 {
            let (width, height) = Panel::window_for(board, scale);
            if width <= MAX_WIDTH && height <= MAX_HEIGHT {
                break;
            }
            scale -= 1;
        }
        scale
    }

    pub fn scaled(mut self, scale: u32) -> Self {
        self.scale = scale;
        self
    }

    /// The whole window `board` would open at `scale`.
    pub const fn window_for(board: Board, scale: u32) -> (i32, i32) {
        match board.bezel {
            Some(bezel) => BezelLayout::new(bezel, board.width, board.height, scale).window_size(),
            None => (board.width * scale as i32, board.height * scale as i32),
        }
    }

    /// The window this panel opens: the whole body when the board has one, and
    /// the panel itself when it has not.
    pub const fn window_size(&self) -> (i32, i32) {
        Panel::window_for(self.board, self.scale)
    }

    /// The panel's own size in window pixels, before any body is drawn round
    /// it. The whole window when the board has no bezel.
    pub const fn size_in_window(&self) -> (i32, i32) {
        (
            self.width * self.scale as i32,
            self.height * self.scale as i32,
        )
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
