//! Where every part of a device body lands, committed per board.
//!
//! `tests/paint.rs` proves the body is *drawn*: every key face holds more than
//! one colour, the panel well differs from the body around it, a held key
//! changes. Those catch **blank**. They do not catch **wrong** — move a key
//! six pixels and all three still pass, while a click lands on its neighbour.
//! A dialog check in `xpui` was loose in exactly that way; its
//! `docs/testing.md` lists it among the tests that passed while wrong.
//!
//! So this pins the arithmetic instead of the pixels: window size, panel well,
//! and every key's face, label **and action**, for all eight boards. The
//! action is the more important half — the regression above was about what a
//! key *sends*, not what it says. Text rather than an
//! image, and not for want of a harness — `xpui-screenshot`'s golden is
//! `BinaryColor`, which is a panel. A body is grey plastic on a grey
//! background, and a picture of one diffs unreadably while telling you nothing
//! a coordinate would not.
//!
//! ```bash
//! UPDATE_SNAPSHOTS=1 cargo test --test geometry     # then read the diff
//! ```

use std::fmt::Write as _;
use std::{env, fs};

use xpui_simulator::{BezelLayout, Board, Panel};

mod devices;

/// The same convention every golden in the organisation uses.
fn compare(name: &str, actual: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));

    let updating = env::var("UPDATE_SNAPSHOTS").is_ok_and(|v| v != "0");
    let existing = fs::read_to_string(&path).ok();

    if updating || existing.is_none() {
        fs::create_dir_all(path.parent().expect("a parent")).expect("create tests/snapshots");
        fs::write(&path, actual).expect("write the golden");
        // A golden written for the first time still fails, so nobody commits
        // one they have never read.
        assert!(
            updating,
            "wrote a new golden at {}. Read it, then run again.",
            path.display()
        );
        return;
    }

    let expected = existing.expect("checked above");
    assert!(
        expected == actual,
        "geometry for `{name}` changed.\n\n--- committed\n{expected}\n--- now\n{actual}\n\
         Re-run with UPDATE_SNAPSHOTS=1 once you have read the difference."
    );
}

/// Every number a body is laid out from, in the order a reader would want it.
fn describe(board: Board) -> String {
    let bezel = board.bezel.expect("this board has a body");
    let panel = Panel::of(board);
    let layout = BezelLayout::new(bezel, panel.width, panel.height, panel.scale);
    let (width, height) = layout.window_size();

    let mut out = String::new();
    writeln!(out, "board       {} ({})", board.name, board.slug).expect("write");
    writeln!(
        out,
        "panel       {}x{} at scale {}",
        panel.width, panel.height, panel.scale
    )
    .expect("write");
    writeln!(out, "window      {width}x{height}").expect("write");

    // `to_window` speaks the bezel's units — tenths of a millimetre from the
    // body's top-left — so the well is `panel_origin`, not (0,0). Passing
    // (0,0) reports the body's own corner, which is (0,0) for every board and
    // therefore looks plausible in all eight goldens while measuring nothing.
    let origin = layout.to_window(bezel.panel_origin);
    let size = layout.to_window_size(bezel.panel_size);
    writeln!(
        out,
        "well        ({},{}) {}x{}",
        origin.x, origin.y, size.0, size.1
    )
    .expect("write");

    // `key_face`, the function the painter calls — not a second copy of its
    // arithmetic. Two implementations disagreed by a pixel on four of the X3's
    // keys, and the golden recorded coordinates nothing drew.
    for button in bezel.buttons {
        let (face, size) = layout.key_face(button.centre, button.size);
        // The action as well as the label. A board whose keys are painted
        // correctly and *send* the wrong thing is the regression this project
        // has actually shipped — every hint label one key off, on two boards,
        // with the suite green.
        writeln!(
            out,
            "key         ({},{}) {}x{} \"{}\" -> {:?}",
            face.x, face.y, size.0, size.1, button.label, button.action
        )
        .expect("write");
    }
    out
}

/// One golden per board. A board that gains, loses or moves a key shows up as
/// a line in a diff rather than as a click that lands next door.
#[test]
fn every_board_lays_its_body_out_the_same_way_it_did() {
    let mut described = 0;
    for board in devices::ALL {
        if board.bezel.is_none() {
            continue;
        }
        compare(board.slug, &describe(board));
        described += 1;
    }

    // Counted, because `continue` is silent. A board that lost its bezel would
    // drop out of this loop and leave its golden behind, never compared again
    // and never reported — which reads exactly like a board that still passes.
    assert_eq!(
        described,
        devices::ALL.len(),
        "every board here is expected to have a body; one stopped having one"
    );
}
