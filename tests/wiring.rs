//! That the loop actually uses what it was given.
//!
//! `presses.rs` drives [`Keypad`] directly, which proves what a keypad does
//! and nothing about whether [`Simulator::run`] ever calls one. That gap is
//! not hypothetical: with it open, `Simulator::keys` could ignore its argument
//! and the loop could stop asking for anything due, and every other test in
//! this repository would still pass.
//!
//! So these open a real window — under SDL's dummy driver, so there is no
//! display anywhere — and run the real loop for a fixed number of frames.
//!
//! ```bash
//! SDL_VIDEODRIVER=dummy cargo test -p xpui-simulator --test wiring
//! ```

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use xpui::screen::Screen;
use xpui::{Button, NavigationScreen, Text, View, vstack};
use xpui_boards_pimoroni as pimoroni;
use xpui_boards_xteink as xteink;
use xpui_simulator::{Keys, Panel, Press, Simulator};

/// One at a time: `Simulator::run` installs the process-wide host, and these
/// count through statics.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// SDL needs a driver even to open a window it will never show.
fn headless() {
    // Safety: single-threaded here, guarded by `SERIAL`, and set before any
    // window is created.
    unsafe { std::env::set_var("SDL_VIDEODRIVER", "dummy") };
}

static DUE_ASKED: AtomicU32 = AtomicU32::new(0);
static TRANSLATED: AtomicU32 = AtomicU32::new(0);

/// Records that it was consulted, and otherwise does nothing at all.
struct Counting;

impl Keys for Counting {
    fn translate(&mut self, press: Press) -> Option<Button> {
        TRANSLATED.fetch_add(1, Ordering::Relaxed);
        Some(press.button)
    }

    fn due(&mut self, _now: u32) -> Option<Button> {
        DUE_ASKED.fetch_add(1, Ordering::Relaxed);
        None
    }
}

struct Blank;

impl Screen for Blank {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("wiring")])
    }

    fn update(&mut self, _message: ()) {}
}

const FRAMES: u32 = 5;

/// The loop asks for anything due, every frame, whether or not a key arrived.
///
/// This is the whole delivery route for a press somebody held back: nothing
/// else ever calls `due`. A loop that stopped asking would leave a badge's
/// select permanently undelivered, and the state machine's own tests would not
/// notice, because they call `due` themselves.
#[test]
fn the_loop_asks_the_keys_it_was_given_for_anything_due() {
    let _guard = serial();
    headless();
    DUE_ASKED.store(0, Ordering::Relaxed);

    Simulator::new(Panel::of(pimoroni::BADGER_2040))
        .frames(FRAMES)
        .keys(Counting)
        .run(Blank);

    let asked = DUE_ASKED.load(Ordering::Relaxed);
    assert!(
        asked >= FRAMES,
        "{FRAMES} frames ran and the keys were asked {asked} times — a press \
         held back is delivered by nothing else, so a loop that does not ask \
         never delivers one"
    );
}

/// And it asks the keys *it was given*, not a default it made itself.
///
/// `Simulator::keys` returning `self` unchanged is a one-character mistake
/// that turns every board's key handling back into the raw one, silently.
#[test]
fn the_keys_the_caller_supplied_are_the_ones_installed() {
    let _guard = serial();
    headless();
    DUE_ASKED.store(0, Ordering::Relaxed);

    Simulator::new(Panel::of(xteink::X4))
        .frames(FRAMES)
        .run(Blank);
    let without = DUE_ASKED.load(Ordering::Relaxed);

    Simulator::new(Panel::of(xteink::X4))
        .frames(FRAMES)
        .keys(Counting)
        .run(Blank);
    let with = DUE_ASKED.load(Ordering::Relaxed);

    assert_eq!(
        without, 0,
        "a simulator given no keys consulted the counting ones anyway"
    );
    assert!(
        with > 0,
        "a simulator given keys never consulted them — whatever `keys` \
         returned, it was not a simulator holding them"
    );
}
