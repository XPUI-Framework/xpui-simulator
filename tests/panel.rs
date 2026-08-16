//! What size window each board gets, and that the board survives the trip.
//!
//! Both of these were wrong and neither was asserted. `OutputSettingsBuilder
//! ::theme()` quietly defaults `pixel_spacing` to 1, which made every window
//! roughly twice its intended size and halved every mouse coordinate; and the
//! simulator rebuilt the board from its width and height, discarding the name,
//! the refresh time and whether the device had a touchscreen.

use xpui_simulator::{Board, Panel};

/// The window is exactly the panel times the scale. No gaps, no padding.
fn window(panel: Panel) -> (i32, i32) {
    (
        panel.width * panel.scale as i32,
        panel.height * panel.scale as i32,
    )
}

#[test]
fn every_board_gets_a_window_that_fits_a_modest_display() {
    for board in Board::ALL {
        let (width, height) = window(Panel::of(board));
        assert!(
            width <= 1200 && height <= 900,
            "{}: {}x{} window is bigger than the budget — it may not fit, and \
             a window that opens off screen cannot always be dragged back",
            board.name,
            width,
            height
        );
    }
}

#[test]
fn a_small_panel_is_scaled_up_and_a_large_one_is_not() {
    assert_eq!(
        Panel::of(Board::BADGER_2040).scale,
        3,
        "a 296x128 strip at 1:1 is a postage stamp"
    );
    assert_eq!(
        Panel::of(Board::READER_LANDSCAPE).scale,
        1,
        "an 800x480 panel is already a reasonable window"
    );
}

/// Scaling must not key off height alone: an 800x480 panel has a *shorter*
/// height than a 480x800 one and would be blown up to 1600 pixels wide.
#[test]
fn scaling_considers_width_too() {
    let landscape = Panel::of(Board::READER_LANDSCAPE);
    assert_eq!(window(landscape), (800, 480));
}

#[test]
fn the_panel_carries_the_board_it_was_built_from() {
    for board in Board::ALL {
        let panel = Panel::of(board);
        assert_eq!(
            panel.board, board,
            "{}: the board must reach the backend whole — a rebuilt one loses \
             its name, its refresh time and its touch capability",
            board.name
        );
    }
}

/// The Badger has no touchscreen, and simulating one would let a screen ship
/// with tap-only controls that are unreachable on the hardware.
#[test]
fn a_board_without_touch_stays_without_touch() {
    assert!(!Panel::of(Board::BADGER_2040).board.touch);
}
