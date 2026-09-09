//! Which board the window is showing, and changing it without restarting.
//!
//! The window cannot resize: `MultiWindow` fixes its SDL window and its
//! streaming texture in the constructor, and there is no resize API. So the
//! window is opened once, big enough for the largest board any key can reach,
//! and every smaller one is letterboxed into the middle of it. Everything that
//! follows from that — how far zoom may go, where the device sits, which
//! backend is installed — is decided here, and here only, so the painter and
//! the event loop measure against one set of numbers.
//!
//! Nothing in this file touches SDL. That is deliberate: switching board is
//! the part worth testing, and a test that needs a window is a test nobody
//! runs on a CI machine.

use embedded_graphics::geometry::Size as PixelSize;

use xpui::Point;
use xpui_boards_core::Board;
use xpui_chrome::{Labels, Metrics};
use xpui_eg::{Backend, Fonts, Palette};

use crate::controls::Control;
use crate::layout::BezelLayout;
use crate::panel::{Panel, PanelDisplay};

/// The most window pixels one panel pixel may occupy.
///
/// A ceiling above the real one: the window is what actually limits zoom,
/// and on a reader-sized panel that stops at life size. This number binds
/// only on the small panels.
const MAX_SCALE: u32 = 6;

/// What the window is showing, and everything that can change about it.
pub struct Session {
    /// Every board the board keys cycle through, in order.
    boards: Vec<Board>,
    /// One backend per board, built when that board is first shown.
    ///
    /// Each backend is leaked — [`xpui::host::install`] takes a `&'static` —
    /// so keyed by board the set is bounded at one per entry, rather than a
    /// panel's worth of pixels per press of B.
    panels: Vec<Option<&'static Backend<PanelDisplay>>>,
    /// How many have actually been built, which is how much has been leaked.
    ///
    /// Counted rather than derived from `panels`: a switch that leaked a fresh
    /// backend and then stored it in the same slot leaves that vector looking
    /// exactly as it should.
    built: usize,
    index: usize,
    scale: u32,
    /// Whether the device body is drawn around the panel.
    body: bool,
    /// The window, fixed when it was opened.
    window: (i32, i32),
}

impl Session {
    /// Opens on `panel`, and installs the backend behind it.
    ///
    /// The cycle is that one board: this crate knows no devices, and the
    /// panel on screen is always somewhere the board keys can be.
    /// [`cycling`](Session::cycling) is how a caller offers more.
    pub fn new(panel: Panel) -> Session {
        Session::cycling(panel, &[panel.board])
    }

    /// Opens on `panel`, with `boards` as the cycle the board keys walk.
    ///
    /// The order is the caller's, and so is the membership: an application
    /// simulating one panel nobody here has heard of gets the same window and
    /// the same keys as one offering seven.
    pub fn cycling(panel: Panel, boards: &[Board]) -> Session {
        let mut boards: Vec<Board> = boards.to_vec();
        let index = match boards.iter().position(|board| *board == panel.board) {
            Some(index) => index,
            None => {
                // The panel was opened on a board the list does not hold. It
                // joins the cycle rather than being replaced by the first
                // press of B, which is what would otherwise happen: the window
                // is showing something the caller cannot get back to.
                boards.push(panel.board);
                boards.len() - 1
            }
        };

        let mut session = Session {
            window: window_for(&boards, panel),
            panels: vec![None; boards.len()],
            built: 0,
            boards,
            index,
            scale: panel.scale,
            body: true,
        };
        session.show();
        session
    }

    /// The cycle the board keys walk, in order: what `B` will do, said at
    /// startup rather than found by pressing it.
    pub fn boards(&self) -> &[Board] {
        &self.boards
    }

    /// The board on screen.
    pub fn board(&self) -> Board {
        self.boards[self.index]
    }

    /// How many window pixels one panel pixel occupies.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// Whether the device body is being drawn.
    pub fn body_shown(&self) -> bool {
        self.body
    }

    /// The backend the framework is installed on, which is this board's.
    pub fn backend(&self) -> &'static Backend<PanelDisplay> {
        self.panels[self.index].expect("the board on screen has been shown")
    }

    /// How many backends have been built, and so how many have been leaked.
    ///
    /// Exposed so a test can prove the cycling reuses them rather than leaking
    /// one per press.
    pub fn backends_built(&self) -> usize {
        self.built
    }

    /// The window, fixed for the whole session.
    pub fn window_size(&self) -> (i32, i32) {
        self.window
    }

    /// How much of the window the device fills: the body when it is shown, and
    /// the bare panel when it is not.
    pub fn shown_size(&self) -> (i32, i32) {
        shown(self.board(), self.scale, self.body)
    }

    /// Where the device's top-left corner sits, centred in the fixed window.
    pub fn origin(&self) -> Point {
        let (width, height) = self.shown_size();
        Point::new((self.window.0 - width) / 2, (self.window.1 - height) / 2)
    }

    /// The body around the panel, placed in the window — or `None` when this
    /// board has no body or the body is hidden.
    pub fn layout(&self) -> Option<BezelLayout> {
        if !self.body {
            return None;
        }
        let board = self.board();
        board.bezel.map(|bezel| {
            BezelLayout::new(bezel, board.width, board.height, self.scale).at(self.origin())
        })
    }

    /// Where the panel's top-left corner sits in the window.
    pub fn panel_offset(&self) -> Point {
        self.layout()
            .map_or_else(|| self.origin(), |layout| layout.panel_offset())
    }

    /// Applies a control, and answers whether the window has to be rebuilt.
    ///
    /// `false` means the key changed nothing — the last board in the cycle is
    /// still a board, and zoom that is already as far in as the window allows
    /// stays there — so the caller can skip a repaint rather than flashing the
    /// panel for nothing.
    pub fn apply(&mut self, control: Control) -> bool {
        let before = (self.index, self.scale, self.body);
        match control {
            Control::NextBoard => self.index = (self.index + 1) % self.boards.len(),
            Control::PreviousBoard => {
                self.index = (self.index + self.boards.len() - 1) % self.boards.len()
            }
            Control::ZoomIn => self.scale += 1,
            Control::ZoomOut => self.scale = self.scale.saturating_sub(1),
            Control::ToggleBody => self.body = !self.body,
            // Not a change to what is shown. It is in the same enum because it
            // is in the same key table, and a second enum for one variant
            // would be two places to keep in step.
            Control::Screenshot => return false,
        }

        // After every control, not only after zoom: a board with a larger body
        // may not fit the scale the last one was at, and showing the body
        // again costs the room its shell takes up.
        self.scale = self.scale.clamp(1, self.ceiling());

        if (self.index, self.scale, self.body) == before {
            return false;
        }
        if self.index != before.0 {
            self.show();
        }
        true
    }

    /// The largest scale this board may be shown at: the largest whose window
    /// still fits the one that was opened.
    ///
    /// Read against what is *currently* shown, so hiding the body of a small
    /// device in a large shell buys back the room its shell took.
    pub fn ceiling(&self) -> u32 {
        let board = self.board();
        let mut scale = MAX_SCALE;
        while scale > 1 {
            let (width, height) = shown(board, scale, self.body);
            if width <= self.window.0 && height <= self.window.1 {
                break;
            }
            scale -= 1;
        }
        scale
    }

    /// Installs the backend for the board now on screen, building it if this
    /// is the first time that board has been shown.
    fn show(&mut self) {
        let backend = match self.panels[self.index] {
            Some(backend) => backend,
            None => {
                let board = self.boards[self.index];
                // Sized from the board so the chrome matches the panel — the
                // whole reason the presets exist. A 296x128 strip laid out
                // with 480x800 chrome shows no list rows at all.
                let display =
                    PanelDisplay::new(PixelSize::new(board.width as u32, board.height as u32));
                // The simulator stands in for a firmware, so it wires a
                // backend the way one does: measurements and words from the
                // panel, keys and the Left/Right pair from the hardware.
                let metrics = Metrics::for_device(
                    board.width,
                    board.height,
                    board.ui_scale_percent,
                    !board.touch,
                );
                // `INK_IS_ON` is what `screenshot.rs` reads pixels by and
                // what `window_settings`'s theme maps: change one, change all.
                let backend = Backend::new(display, Palette::INK_IS_ON)
                    .with_metrics(metrics)
                    .with_labels(Labels::for_panel(board.width, board.height))
                    .with_keys(board.keys)
                    .with_left_right_keys(board.has_left_right_keys())
                    .with_fonts(Fonts::for_metrics(&metrics))
                    .leaked();
                self.built += 1;
                self.panels[self.index] = Some(backend);
                backend
            }
        };

        // Safety: one thread, and no frame in flight. The events that ask for
        // this are drained before the frame they arrived in measures or paints
        // anything, and the simulator has no second render task. The old
        // backend stays `&'static` and valid, so nothing that read it is left
        // dangling.
        unsafe { xpui::host::install(backend) };
    }
}

/// How much of the window `board` fills at `scale`.
fn shown(board: Board, scale: u32, body: bool) -> (i32, i32) {
    match body {
        true => Panel::window_for(board, scale),
        // `window_for` answers with the body when the board has one, and this
        // is the case where it has one and it is hidden.
        false => (board.width * scale as i32, board.height * scale as i32),
    }
}

/// The window that holds every board this session can reach.
///
/// The largest each of them opens at its own default scale, and never smaller
/// than the panel the caller asked for — a caller that passed
/// `Panel::of(board).scaled(4)` meant it, and a window that clipped it would
/// be worse than one with room to spare.
fn window_for(boards: &[Board], start: Panel) -> (i32, i32) {
    let mut size = start.window_size();
    for board in boards {
        let (width, height) = Panel::window_for(*board, Panel::scale_for(*board));
        size = (size.0.max(width), size.1.max(height));
    }
    size
}
