//! Changing the simulator while it runs — with no window anywhere.
//!
//! Switching board is the part worth asserting and the part hardest to try:
//! it replaces the installed host mid-session, and the screen stack has to
//! come through unharmed and be re-measured against the panel it landed on.
//! [`Session`] is deliberately free of SDL so that all of that can be driven
//! from an ordinary `cargo test`.
//!
//! The host is process-wide, so these run one at a time.

use std::sync::{Mutex, MutexGuard};

use embedded_graphics::prelude::*;

use xpui::{App, Divider, Renderer, Screen, Size, View, vstack};
use xpui_simulator::{Board, Control, Panel, PanelDisplay, Session};

/// `xpui::host::install` writes a static, and Cargo runs tests in parallel.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// -- two screens made of horizontal rules ----------------------------------
//
// A rule spans whatever width the layout gives it and leaves ink on exactly
// one row, so a panel painted by one of these answers two questions at once:
// how many rules were drawn says *which* screen was on top, and how far they
// reach says what width it was measured against.

struct OneRule;

impl Screen for OneRule {
    type Message = ();

    fn body(&self) -> impl View<()> {
        vstack![8; Divider::new()]
    }

    fn update(&mut self, _message: ()) {}
}

struct ThreeRules;

impl Screen for ThreeRules {
    type Message = ();

    fn body(&self) -> impl View<()> {
        vstack![8; Divider::new(), Divider::new(), Divider::new()]
    }

    fn update(&mut self, _message: ()) {}
}

/// How many rows of the panel have any ink on them.
fn rules_painted(session: &Session) -> usize {
    session
        .backend()
        .with_display(|display| rows(display).len())
}

/// The rightmost column with ink, or `None` on a blank panel.
fn ink_reaches(session: &Session) -> Option<i32> {
    session.backend().with_display(|display| {
        let size = display.bounding_box().size;
        (0..size.width as i32)
            .rev()
            .find(|x| (0..size.height as i32).any(|y| display.get_pixel(Point::new(*x, y)).is_on()))
    })
}

fn rows(display: &PanelDisplay) -> Vec<i32> {
    let size = display.bounding_box().size;
    (0..size.height as i32)
        .filter(|y| (0..size.width as i32).any(|x| display.get_pixel(Point::new(x, *y)).is_on()))
        .collect()
}

// -- the board key ---------------------------------------------------------

/// The whole point of the key: the same screen, on another panel, without
/// navigating back to it.
///
/// It starts on the narrowest board and walks until it reaches a wider one,
/// because the direction is what makes the assertion bite: a screen that was
/// *not* re-measured keeps the narrow board's width, and on a wider panel that
/// leaves its rules stopping short. Going the other way, the same fault is
/// hidden by the panel clipping the overhang.
///
/// A stack that was rebuilt would come back one screen deep, painting a single
/// rule instead of three.
#[test]
fn switching_board_keeps_the_screen_stack_and_re_measures_it() {
    let _guard = serial();

    let start = Board::TUFTY_2040;
    let mut session = Session::new(Panel::of(start));
    let mut app = App::new(OneRule);
    app.push(ThreeRules);
    app.render();

    assert_eq!(app.depth(), 2);
    assert_eq!(
        rules_painted(&session),
        3,
        "the pushed screen is the one on"
    );
    assert_eq!(
        ink_reaches(&session),
        Some(start.width - 1),
        "a rule spans the panel it was measured against"
    );

    for _ in 0..Board::ALL.len() {
        assert!(
            session.apply(Control::NextBoard),
            "the next board is a change"
        );
        if session.board().width > start.width {
            break;
        }
    }
    let landed = session.board();
    assert!(
        landed.width > start.width,
        "no board is wider than the {}, so this test cannot tell a re-measured \
         layout from a clipped one",
        start.name
    );

    app.render();

    assert_eq!(
        app.depth(),
        2,
        "the stack is the app's, and a new backend is no reason to lose it"
    );
    assert_eq!(
        rules_painted(&session),
        3,
        "the screen still on top is the one that was on top"
    );
    assert_eq!(
        Renderer::screen_size(),
        Size::new(landed.width, landed.height),
        "the framework is measuring against the panel that is now shown"
    );
    assert_eq!(
        ink_reaches(&session),
        Some(landed.width - 1),
        "and the screen was laid out against it, not against the panel it left"
    );
}

/// Every board, twice round, builds one backend each and no more.
///
/// The backends are leaked — `xpui::host::install` takes a `&'static` — so a
/// switch that built a fresh one would leak a panel's worth of pixels per
/// press, and cycling for an hour would exhaust the process.
#[test]
fn cycling_the_boards_reuses_their_backends() {
    let _guard = serial();

    let mut session = Session::new(Panel::DEFAULT);
    for _ in 0..Board::ALL.len() * 2 {
        session.apply(Control::NextBoard);
    }

    assert_eq!(
        session.backends_built(),
        Board::ALL.len(),
        "two laps built {} backends for {} boards",
        session.backends_built(),
        Board::ALL.len()
    );
}

/// Backwards is the same cycle, the other way.
#[test]
fn shift_walks_the_boards_the_other_way() {
    let _guard = serial();

    let mut session = Session::new(Panel::DEFAULT);
    let start = session.board();

    session.apply(Control::NextBoard);
    let forward = session.board();
    session.apply(Control::PreviousBoard);
    assert_eq!(session.board(), start, "back where it started");

    session.apply(Control::PreviousBoard);
    assert_ne!(
        session.board(),
        forward,
        "the two keys must not walk the same way"
    );
}

// -- zoom ------------------------------------------------------------------

/// Zoom is a window concern. Nothing the screen is laid out against may move.
///
/// If the scale ever leaks into the board, a screen re-lays-out when you zoom
/// — and what you were inspecting is no longer what you were inspecting.
#[test]
fn zoom_does_not_change_the_panel() {
    let _guard = serial();

    // The Badger opens tripled, so it has room to zoom out and back.
    let mut session = Session::new(Panel::of(Board::BADGER_2040));
    let mut app = App::new(OneRule);
    app.render();

    let scale = session.scale();
    let reach = ink_reaches(&session);
    assert_eq!(reach, Some(Board::BADGER_2040.width - 1));

    assert!(
        session.apply(Control::ZoomOut),
        "there was room to zoom out"
    );
    assert_ne!(session.scale(), scale, "the scale moved");
    app.render();

    assert_eq!(
        Renderer::screen_size(),
        Size::new(Board::BADGER_2040.width, Board::BADGER_2040.height),
        "the panel is the same number of pixels at any scale"
    );
    assert_eq!(
        ink_reaches(&session),
        reach,
        "and the screen was laid out against the same width"
    );
}

/// Zoom stops where the window does, and never below life size.
#[test]
fn zoom_is_clamped_at_both_ends() {
    let _guard = serial();

    let mut session = Session::new(Panel::of(Board::BADGER_2040));
    for _ in 0..12 {
        session.apply(Control::ZoomOut);
    }
    assert_eq!(session.scale(), 1, "life size is the floor");
    assert!(
        !session.apply(Control::ZoomOut),
        "a key that changes nothing must say so, or the panel flashes for it"
    );

    for _ in 0..12 {
        session.apply(Control::ZoomIn);
    }
    assert_eq!(
        session.scale(),
        session.ceiling(),
        "as far in as the window allows, and no further"
    );
    assert!(!session.apply(Control::ZoomIn));
}

// -- the fixed window ------------------------------------------------------

/// The window is opened once and never resizes, so everything shown has to fit
/// inside it — every board, at every scale any key can reach, with the body
/// shown and hidden.
#[test]
fn nothing_ever_outgrows_the_window() {
    let _guard = serial();

    let mut session = Session::new(Panel::DEFAULT);
    let (window_width, window_height) = session.window_size();

    for _ in 0..Board::ALL.len() * 2 {
        // Twice: once with the body shown, once without, ending as it began.
        for _ in 0..2 {
            for _ in 0..12 {
                session.apply(Control::ZoomIn);
            }
            let (width, height) = session.shown_size();
            assert!(
                width <= window_width && height <= window_height,
                "{} at {}x is {width}x{height}, past the {window_width}x{window_height} window",
                session.board().name,
                session.scale()
            );
            session.apply(Control::ToggleBody);
        }
        session.apply(Control::NextBoard);
    }
}

/// A device smaller than the window sits in the middle of it, not in a corner.
#[test]
fn the_panel_stays_centred() {
    let _guard = serial();

    let mut session = Session::new(Panel::DEFAULT);
    let (window_width, window_height) = session.window_size();

    for _ in 0..Board::ALL.len() {
        for _ in 0..2 {
            let origin = session.origin();
            let (width, height) = session.shown_size();
            let (right, below) = (
                window_width - (origin.x + width),
                window_height - (origin.y + height),
            );

            assert!(
                (origin.x - right).abs() <= 1 && (origin.y - below).abs() <= 1,
                "{}: {}px left of it and {right}px right, {}px above and {below}px below",
                session.board().name,
                origin.x,
                origin.y
            );
            session.apply(Control::ToggleBody);
        }
        session.apply(Control::NextBoard);
    }
}

/// Hiding the body letterboxes the panel. It does not resize the window, and
/// there is no API that could.
#[test]
fn hiding_the_body_letterboxes_rather_than_resizes() {
    let _guard = serial();

    let mut session = Session::new(Panel::of(Board::X4));
    let window = session.window_size();
    let with_body = session.shown_size();
    assert!(session.body_shown() && session.layout().is_some());

    assert!(session.apply(Control::ToggleBody));

    assert!(!session.body_shown(), "the key was heard");
    assert!(session.layout().is_none(), "and the body is not drawn");
    assert_eq!(session.window_size(), window, "the window cannot resize");
    assert_eq!(
        session.shown_size(),
        (Board::X4.width, Board::X4.height),
        "what is left is the bare panel"
    );
    assert!(
        session.shown_size().0 < with_body.0,
        "which is smaller than the device around it, hence the letterbox"
    );
}

/// A board nobody here has heard of still gets a window it fits in, and joins
/// the cycle rather than being lost at the first press of B.
#[test]
fn a_board_of_someone_elses_is_carried_too() {
    let _guard = serial();

    let odd = Board::custom("odd", 1100, 200, false);
    let mut session = Session::new(Panel::of(odd));

    let (width, height) = session.window_size();
    assert!(
        width >= 1100 && height >= 200,
        "the window has to hold the panel it was opened for"
    );

    let mut seen = false;
    for _ in 0..=Board::ALL.len() {
        session.apply(Control::NextBoard);
        seen |= session.board() == odd;
    }
    assert!(seen, "it is still in the cycle a full lap later");
}
