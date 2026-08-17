//! Raw presses, and what a caller may do to them on the way past.
//!
//! [`Keypad`] is the whole of what the event loop does to a key, so driving it
//! directly is driving the real path — the window is the only part missing,
//! and the window has no opinion about any of this.
//!
//! Read back through [`Input`], because that is where the framework reads: a
//! press that does not show up there did not happen as far as a screen is
//! concerned, whatever the backend was told.

use std::sync::{Mutex, MutexGuard};

use xpui::Button;
use xpui::host::Input;
use xpui_simulator::{Board, Keypad, Keys, Panel, Press, Raw, Session};

/// `Session::new` installs the process-wide host, so one test at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn keypad(keys: impl Keys + 'static) -> Keypad {
    Keypad::new(Box::new(keys))
}

// -- three doubles, each saying one thing --------------------------------

/// Renames every press to `Back`.
struct Renames;

impl Keys for Renames {
    fn translate(&mut self, _press: Press) -> Option<Button> {
        Some(Button::Back)
    }
}

/// Eats every press and never gives one back.
struct Eats;

impl Keys for Eats {
    fn translate(&mut self, _press: Press) -> Option<Button> {
        None
    }
}

/// Presses `Confirm` from a timer, once, at `at`.
struct Timer {
    at: u32,
    fired: bool,
}

impl Keys for Timer {
    fn translate(&mut self, _press: Press) -> Option<Button> {
        None
    }

    fn due(&mut self, now: u32) -> Option<Button> {
        if self.fired || now < self.at {
            return None;
        }
        self.fired = true;
        Some(Button::Confirm)
    }
}

// -- the seam --------------------------------------------------------------

#[test]
fn a_press_arrives_untouched_when_nothing_reads_it() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::X4));
    let mut keys = keypad(Raw);

    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::X4);

    assert!(
        Input::was_pressed(Button::Confirm),
        "the default reads nothing into a press and delivers the key itself"
    );
}

#[test]
fn a_translated_press_is_released_as_what_it_became() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::X4));
    let mut keys = keypad(Renames);

    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::X4);

    assert!(
        Input::is_pressed(Button::Back),
        "the translation is what went down"
    );
    assert!(
        !Input::is_pressed(Button::Confirm),
        "the key that was physically struck never reached the framework"
    );

    // The key comes up. What has to come up with it is `Back` — the button
    // that actually went down — and not `Confirm`, which never did.
    session.backend().begin_frame(1);
    keys.up(session.backend(), Button::Confirm);

    assert!(
        !Input::is_pressed(Button::Back),
        "releasing the key released what it had become; \
         letting go of the raw button instead leaves Back held down for the \
         rest of the session, auto-repeating"
    );
}

#[test]
fn a_swallowed_press_reaches_nothing() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::X4));
    let mut keys = keypad(Eats);

    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::X4);

    assert!(
        !Input::was_pressed(Button::Confirm),
        "a press the caller swallowed is not delivered"
    );
    assert!(
        !Input::is_pressed(Button::Confirm),
        "and it is not held either"
    );

    // And the release of a press that never landed releases nothing, rather
    // than reporting a button coming up that never went down.
    session.backend().begin_frame(1);
    keys.up(session.backend(), Button::Confirm);

    assert!(
        !Input::was_released(Button::Confirm),
        "nothing went down, so nothing comes up"
    );
}

#[test]
fn a_press_that_comes_due_arrives_whole() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::BADGER_2040));
    let mut keys = keypad(Timer {
        at: 350,
        fired: false,
    });

    // Swallowed on the way down, and nothing is due yet.
    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::BADGER_2040);
    keys.due(session.backend(), 0);
    assert!(
        !Input::was_pressed(Button::Confirm),
        "it is not due on the frame the key was struck"
    );

    session.backend().begin_frame(349);
    keys.due(session.backend(), 349);
    assert!(
        !Input::was_pressed(Button::Confirm),
        "nor one frame short of its moment"
    );

    session.backend().begin_frame(350);
    keys.due(session.backend(), 350);
    assert!(
        Input::was_pressed(Button::Confirm),
        "the frame it comes due, it is delivered — with no key behind it"
    );
    assert!(
        !Input::is_pressed(Button::Confirm),
        "and delivered complete: an injected press that is never released is \
         held forever, and the runtime auto-repeats a held button"
    );
}

#[test]
fn switching_board_lets_go_of_everything_it_was_holding() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::X4));
    let mut keys = keypad(Raw);

    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::PageForward, 0, Board::X4);
    assert!(Input::is_pressed(Button::PageForward));

    session.backend().begin_frame(1);
    keys.release_all(session.backend());

    assert!(
        !Input::is_pressed(Button::PageForward),
        "a key held through a board switch is let go of, or it stays down on \
         a device that is no longer there"
    );
}

/// Renames the first press of a key, then the next differently, so the two
/// are told apart by what they released.
struct RenamesInTurn {
    seen: u32,
}

impl Keys for RenamesInTurn {
    fn translate(&mut self, _press: Press) -> Option<Button> {
        self.seen += 1;
        Some(if self.seen == 1 {
            Button::Back
        } else {
            Button::Left
        })
    }
}

#[test]
fn a_second_press_with_no_release_replaces_the_first() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::X4));
    let mut keys = keypad(RenamesInTurn { seen: 0 });

    // The same key twice with nothing between: an auto-repeat that got
    // through, or a mouse clicking the drawn key while the keyboard holds it.
    // Each press translated to something different, so what the single
    // release lets go of says how many rows the table was holding.
    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::X4);
    keys.down(session.backend(), Button::Confirm, 1, Board::X4);

    session.backend().begin_frame(2);
    keys.up(session.backend(), Button::Confirm);

    assert!(
        !Input::is_pressed(Button::Back),
        "the first meaning was let go of when the second press replaced it"
    );
    assert!(
        !Input::is_pressed(Button::Left),
        "and the second by the release — one key is one row in the table, \
         however many presses arrive for it, or a meaning nothing releases \
         stays down and auto-repeats"
    );
}

#[test]
fn a_board_switch_tells_the_keys_to_forget() {
    let _guard = serial();
    let session = Session::new(Panel::of(Board::BADGER_2040));
    let mut keys = keypad(Forgetful::default());

    session.backend().begin_frame(0);
    keys.down(session.backend(), Button::Confirm, 0, Board::BADGER_2040);
    keys.release_all(session.backend());

    session.backend().begin_frame(1);
    keys.due(session.backend(), 10_000);

    assert!(
        !Input::was_pressed(Button::Confirm),
        "a select held back on the badge arrived on the board switched to, \
         which never had the key pressed"
    );
}

/// Holds a press back forever, unless told to forget it.
#[derive(Default)]
struct Forgetful {
    pending: bool,
}

impl Keys for Forgetful {
    fn translate(&mut self, _press: Press) -> Option<Button> {
        self.pending = true;
        None
    }

    fn due(&mut self, _now: u32) -> Option<Button> {
        self.pending.then_some(Button::Confirm)
    }

    fn reset(&mut self) {
        self.pending = false;
    }
}
