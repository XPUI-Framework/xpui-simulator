//! What a raw press becomes before the framework sees it.
//!
//! Hardware sends presses. What two of them close together *mean* is the
//! firmware's decision, and a simulator that decides for it is standing in for
//! something the hardware does not do. So the presses arrive raw, and a caller
//! that wants to read something into them says so here.
//!
//! Two directions, deliberately not one:
//!
//! - **A translation** ([`Keys::translate`]) — a key went down and this is what
//!   it means. The release of that same key follows its translation, which is
//!   the part a caller should not have to remember: a press translated to
//!   `Back` is released as `Back`, never as the key that was physically let go.
//! - **An injection** ([`Keys::due`]) — a press with no key behind it, produced
//!   by a timer. Delivered as a complete press *and* release in one frame,
//!   because there is no finger to lift later and a button left held would
//!   auto-repeat forever.
//!
//! A caller returning a button from the first has renamed a press. One
//! returning a button from the second has invented one. Those are different
//! things and they read differently at the call site.

use xpui::Button;
use xpui_boards::Board;
use xpui_eg::Backend;

use crate::panel::PanelDisplay;

/// A key going down, and what was true when it did.
///
/// A struct rather than three arguments so that adding a fact later does not
/// break every implementation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Press {
    /// The button the hardware sent — the key's own identity, before anybody
    /// reads anything into it.
    pub button: Button,
    /// Milliseconds since the run started, the same reading the framework is
    /// given this frame. A window measured against a different clock closes on
    /// the wrong frame.
    pub now: u32,
    /// The device this press came from.
    ///
    /// Here because what a press means can depend on how many keys the board
    /// has — a row of three has no Back key and has to find one somewhere. It
    /// is passed per press rather than fixed at construction because the
    /// simulator can switch boards while it runs, and a decision made for the
    /// board that was there is the wrong decision for the board that is.
    pub board: Board,
}

/// Reads meaning into raw presses.
///
/// Every method has a default that does nothing, so an implementation states
/// only what it changes. The default of the whole trait is [`Raw`]: presses go
/// through untouched.
pub trait Keys {
    /// What `press` means, or `None` to swallow it.
    ///
    /// This is a *translation*: whatever comes back is what gets pressed, and
    /// what gets released when that same physical key comes up. Swallowing is
    /// swallowing — the press is gone unless [`due`](Keys::due) later produces
    /// one, which is the whole reason that method exists.
    fn translate(&mut self, press: Press) -> Option<Button> {
        Some(press.button)
    }

    /// A press that has come due, with no key behind it.
    ///
    /// Asked once a frame, whether or not anything was pressed. This is an
    /// *injection*: it is delivered as a press and its release together, so a
    /// caller does not have to arrange for the second half.
    fn due(&mut self, now: u32) -> Option<Button> {
        let _ = now;
        None
    }

    /// Everything held is being let go of, and nothing pending should arrive.
    ///
    /// The simulator can change board under a running app, and a press held
    /// back on a device that is no longer here must not land on the one that
    /// replaced it — a badge's delayed select would open a screen on a reader
    /// that never had the key pressed. Hardware has no equivalent, which is
    /// exactly why an implementation has to be told rather than work it out.
    fn reset(&mut self) {}
}

/// Presses reach the framework as they arrive.
///
/// What every board did before anything could translate, and what a board with
/// a key for everything wants.
pub struct Raw;

impl Keys for Raw {}

/// The keys of a device, with a [`Keys`] reading them.
///
/// [`Simulator::run`](crate::Simulator::run) drives one of these from the
/// window's events. It is public for the same reason [`Touchscreen`] is: what
/// the loop does to a press can then be driven from a test, or from another
/// loop, with no window anywhere.
///
/// The bookkeeping is here rather than in the caller because it is the one
/// part that is easy to get wrong and never noticed: a key translated on the
/// way down has to be released as what it became, or the button it turned into
/// stays held for the rest of the session while the runtime auto-repeats it.
///
/// [`Touchscreen`]: crate::Touchscreen
pub struct Keypad {
    keys: Box<dyn Keys>,
    /// Physical key to what it was delivered as, for keys currently down.
    ///
    /// A list because a person holds one or two keys and a board has at most
    /// seven; the lookup is shorter than the array it would take to avoid it.
    /// A swallowed press is not in here at all — there is nothing to release.
    down: Vec<(Button, Button)>,
}

impl Keypad {
    pub fn new(keys: Box<dyn Keys>) -> Self {
        Keypad {
            keys,
            down: Vec::new(),
        }
    }

    /// A key went down on `board`. Delivers whatever it turned out to mean.
    pub fn down(
        &mut self,
        backend: &Backend<PanelDisplay>,
        button: Button,
        now: u32,
        board: Board,
    ) {
        // A second press with no release between them — a repeat that got
        // through, or a key clicked with the mouse while the keyboard held it.
        // The old meaning is released before the new one is decided, so the
        // table cannot hold two rows for one key.
        self.up(backend, button);

        let Some(meaning) = self.keys.translate(Press { button, now, board }) else {
            return;
        };
        self.down.push((button, meaning));
        backend.press(meaning);
    }

    /// A key came up. Releases what it was delivered as, if it was delivered.
    pub fn up(&mut self, backend: &Backend<PanelDisplay>, button: Button) {
        let Some(at) = self.down.iter().position(|(raw, _)| *raw == button) else {
            return;
        };
        let (_, meaning) = self.down.remove(at);
        backend.release(meaning);
    }

    /// Delivers anything that has come due, as a press and its release.
    pub fn due(&mut self, backend: &Backend<PanelDisplay>, now: u32) {
        let Some(button) = self.keys.due(now) else {
            return;
        };
        backend.press(button);
        backend.release(button);
    }

    /// Releases everything still held, on the backend it was pressed on.
    ///
    /// For a board switch: the key under the finger is on a device that is no
    /// longer here, and so is the backend it went down on.
    pub fn release_all(&mut self, backend: &Backend<PanelDisplay>) {
        for (_, meaning) in self.down.drain(..) {
            backend.release(meaning);
        }
        self.keys.reset();
    }
}
