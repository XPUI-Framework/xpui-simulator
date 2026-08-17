//! What size window each board gets, and that the board survives the trip.
//!
//! Both of these were wrong and neither was asserted. `OutputSettingsBuilder
//! ::theme()` quietly defaults `pixel_spacing` to 1, which made every window
//! roughly twice its intended size and halved every mouse coordinate; and the
//! simulator rebuilt the board from its width and height, discarding the name,
//! the refresh time and whether the device had a touchscreen.

use xpui_simulator::{Board, Panel};

/// The whole window, body included — not the panel times the scale.
///
/// The difference is the point. A board that describes a body opens a window
/// larger than its panel, and a budget checked against the panel alone let the
/// Tufty 2040 — a 320x240 panel in a comparatively large shell — open at
/// 1280x1034 while claiming to be 960x720.
fn window(panel: Panel) -> (i32, i32) {
    panel.window_size()
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

/// A board with a body opens a window larger than its panel; a board without
/// one opens a window that is exactly the panel, as it always did.
#[test]
fn the_window_is_the_body_when_there_is_one_and_the_panel_when_there_is_not() {
    for board in Board::ALL {
        let panel = Panel::of(board);
        let (window_width, window_height) = panel.window_size();
        let (panel_width, panel_height) = panel.size_in_window();

        if board.bezel.is_some() {
            assert!(
                window_width > panel_width && window_height > panel_height,
                "{}: a {window_width}x{window_height} window round a \
                 {panel_width}x{panel_height} panel leaves nowhere for a body",
                board.name
            );
        } else {
            assert_eq!(
                (window_width, window_height),
                (panel_width, panel_height),
                "{}: no body was described, so the window is the panel",
                board.name
            );
        }
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
        Panel::of(Board::X4).scale,
        1,
        "an 800x480 panel is already a reasonable window"
    );
}

/// Scaling must consider width, not height alone.
///
/// The four real boards cannot show this: every one of them gets the same
/// scale either way, so a test over `Board::ALL` passes with the width clause
/// deleted. It takes a panel that is wide and short — where width is the
/// binding constraint and height is not — for the difference to appear at all.
#[test]
fn a_wide_short_panel_is_limited_by_its_width() {
    // 1100x200: at 3x that is 3300 wide, far past any window budget, while its
    // height at 3x is only 600 and would pass a height-only check.
    let wide = Board::custom("wide", 1100, 200, false);
    let panel = Panel::of(wide);

    assert_eq!(
        panel.scale,
        1,
        "a 1100x200 panel scaled to {}x would be {} pixels wide",
        panel.scale,
        1100 * panel.scale
    );
    assert!(window(panel).0 <= 1200);
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

/// The gap between panel pixels must be zero.
///
/// `OutputSettingsBuilder::theme()` does `pixel_spacing.get_or_insert(1)` as a
/// side effect, and the window is sized `size * scale + (size - 1) * spacing`.
/// One pixel of gap therefore roughly doubles the window and halves every mouse
/// coordinate, because the pitch used to unmap a click is `scale + spacing`.
#[test]
fn there_is_no_gap_between_panel_pixels() {
    for scale in 1..=3 {
        let settings = xpui_simulator::window_settings(scale);
        assert_eq!(
            settings.pixel_spacing, 0,
            "scale {scale} left a gap between pixels"
        );
        assert_eq!(settings.scale, scale, "the requested scale did not survive");
    }
}

/// Ink must reach the window dark, on paper, rather than white on black.
///
/// The default theme is an identity map, and a binary display sends `On` —
/// which is ink — through as white. So leaving the theme alone renders the
/// panel inverted, which on an e-ink simulator reads as a bug.
#[test]
fn the_window_paints_ink_on_paper() {
    use embedded_graphics::prelude::RgbColor;
    use embedded_graphics_simulator::BinaryColorTheme;

    let BinaryColorTheme::Custom {
        color_on,
        color_off,
    } = xpui_simulator::window_settings(1).theme
    else {
        panic!("the theme must map the two colours explicitly, not identity");
    };

    let luma =
        |c: embedded_graphics::pixelcolor::Rgb888| c.r() as u32 + c.g() as u32 + c.b() as u32;
    assert!(
        luma(color_on) < luma(color_off),
        "ink ({color_on:?}) must be darker than paper ({color_off:?})"
    );
}
