//! The keyboard map, pinned against the page that documents it.
//!
//! `docs/running.md` told readers Escape was Back for as long as it was wrong:
//! `embedded-graphics-simulator` converts Escape to `SimulatorEvent::Quit`
//! upstream, so no key press ever arrives to be mapped. Rather than fix the
//! sentence and hope, the table is now read out of the file and checked.

use xpui::Button;
use xpui_simulator::{Control, Keycode, button_for, control_for};

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

/// The page must say what the code does, in both columns.
///
/// Asserting only that a key name appears somewhere leaves the meaning column
/// free to say anything: the page could claim Backspace is Confirm and still
/// pass. So each row is found by its key and then read for the button it
/// promises.
#[test]
fn the_page_says_what_each_key_does() {
    let page = include_str!("../docs/running.md");

    for (label, _, expected) in DOCUMENTED {
        let Some(button) = expected else { continue };

        let row = page
            .lines()
            .filter(|line| line.starts_with('|'))
            .find(|line| {
                let key_column = line.split('|').nth(1).unwrap_or_default();
                key_column.contains(label)
            })
            .unwrap_or_else(|| panic!("docs/running.md has no table row for {label}"));

        let meaning = row.split('|').nth(2).unwrap_or_default();
        let name = format!("Button::{button:?}");
        assert!(
            meaning.contains(&name),
            "docs/running.md says {label} does {meaning:?}, but the code maps \
             it to {name}"
        );
    }
}

/// The reason Escape is not Back has to stay on the page, or someone will
/// helpfully add it back.
#[test]
fn the_page_keeps_explaining_escape() {
    let page = include_str!("../docs/running.md");
    assert!(page.contains("Escape cannot be Back"));
}

// -- the keys that change the simulator rather than driving the screen ------

/// Every row of the live-control table in `docs/running.md`, as `(the doc's key
/// column, the keycode, whether shift was down, what the code must do)`.
///
/// Zoom has several keycodes per row on purpose: `+` is shift-and-equals on
/// most layouts and its own key on a numeric pad, and a table that recognised
/// only one of them would work on the machine it was written on.
const CONTROLS: &[(&str, Keycode, bool, Control)] = &[
    ("B", Keycode::B, false, Control::NextBoard),
    ("Shift+B", Keycode::B, true, Control::PreviousBoard),
    ("+", Keycode::Equals, false, Control::ZoomIn),
    ("+", Keycode::Equals, true, Control::ZoomIn),
    ("+", Keycode::Plus, false, Control::ZoomIn),
    ("+", Keycode::KpPlus, false, Control::ZoomIn),
    ("-", Keycode::Minus, false, Control::ZoomOut),
    ("-", Keycode::KpMinus, false, Control::ZoomOut),
    ("E", Keycode::E, false, Control::ToggleBody),
    ("S", Keycode::S, false, Control::Screenshot),
];

#[test]
fn the_code_does_what_the_page_says_about_the_controls() {
    for (label, key, shift, expected) in CONTROLS {
        assert_eq!(
            control_for(*key, *shift),
            Some(*expected),
            "docs/running.md documents {label} — the code disagrees"
        );
    }
}

/// The table has to cover every control, not a subset of them. A variant with
/// no row is a key nobody can find out about.
#[test]
fn every_control_has_a_row_on_the_page() {
    let page = include_str!("../docs/running.md");

    for control in Control::ALL {
        let label = CONTROLS
            .iter()
            .find(|(_, _, _, documented)| *documented == control)
            .map(|(label, ..)| *label)
            .unwrap_or_else(|| panic!("{control:?} is in no row of this test's own table"));

        // The key column, matched whole. `contains` would let "B" find the
        // Backspace row of the button table above it and call that a match.
        let row = page
            .lines()
            .filter(|line| line.starts_with('|'))
            .find(|line| line.split('|').nth(1).unwrap_or_default().trim() == label);

        assert!(
            row.is_some(),
            "docs/running.md has no table row for {label}, which is {control:?}"
        );
    }
}

/// A control key must not also be a button, because the loop checks the
/// controls first and would swallow the press.
#[test]
fn no_control_key_is_also_a_button() {
    for (label, key, _, _) in CONTROLS {
        assert_eq!(
            button_for(*key),
            None,
            "{label} is a control, so mapping it to a button as well means the \
             button can never be pressed"
        );
    }
}

/// And an ordinary key is not a control. Q ends the loop before the controls
/// are consulted, and claiming it here would be a second answer to the same
/// key.
#[test]
fn a_key_that_is_not_a_control_is_not_claimed() {
    for key in [Keycode::A, Keycode::Q, Keycode::Return, Keycode::Up] {
        assert_eq!(control_for(key, false), None, "{key:?} is not a control");
        assert_eq!(control_for(key, true), None, "nor is shift and {key:?}");
    }
}
