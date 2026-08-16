//! The keyboard map, pinned against the page that documents it.
//!
//! `docs/running.md` told readers Escape was Back for as long as it was wrong:
//! `embedded-graphics-simulator` converts Escape to `SimulatorEvent::Quit`
//! upstream, so no key press ever arrives to be mapped. Rather than fix the
//! sentence and hope, the table is now read out of the file and checked.

use xpui::Button;
use xpui_simulator::{Keycode, button_for};

/// Every row of the table in `docs/running.md`, as `(the doc's label, what the
/// code must do with it)`.
const DOCUMENTED: &[(&str, Keycode, Option<Button>)] = &[
    ("Up / Down", Keycode::Up, Some(Button::Up)),
    ("Up / Down", Keycode::Down, Some(Button::Down)),
    ("Left / Right", Keycode::Left, Some(Button::Left)),
    ("Left / Right", Keycode::Right, Some(Button::Right)),
    ("Enter", Keycode::Return, Some(Button::Confirm)),
    ("keypad Enter", Keycode::KpEnter, Some(Button::Confirm)),
    ("Space", Keycode::Space, Some(Button::Confirm)),
    ("Backspace", Keycode::Backspace, Some(Button::Back)),
    ("Page Up", Keycode::PageUp, Some(Button::PageBack)),
    ("Page Down", Keycode::PageDown, Some(Button::PageForward)),
];

#[test]
fn the_code_does_what_the_page_says() {
    for (label, key, expected) in DOCUMENTED {
        assert_eq!(
            button_for(*key),
            *expected,
            "docs/running.md documents {label} — the code disagrees"
        );
    }
}

/// The one that was wrong, kept as its own test so the reason survives.
#[test]
fn escape_is_not_back() {
    assert_eq!(
        button_for(Keycode::Escape),
        None,
        "Escape never reaches us as a key press — the simulator crate turns it \
         into Quit first. Mapping it to Back only makes the page lie."
    );
}

#[test]
fn the_page_still_lists_what_the_code_maps() {
    let page = include_str!("../docs/running.md");
    for (label, ..) in DOCUMENTED {
        assert!(
            page.contains(label),
            "the key table in docs/running.md no longer mentions {label}"
        );
    }
    assert!(
        page.contains("Escape cannot be Back"),
        "the page must keep explaining why Escape is not Back, or someone will \
         helpfully add it back"
    );
}
