//! Where a click goes: onto the panel, onto a physical button, or nowhere.
//!
//! The offset is the whole bug surface. Once the panel is inset into a body,
//! every mouse coordinate has to have that inset removed before the framework
//! sees it, and every click that misses the panel has to stop being a touch —
//! a device with no touchscreen would otherwise receive one at a coordinate it
//! cannot physically have.

use xpui::{Button, Point};
use xpui_boards::KeyAction;
use xpui_boards::{Bezel, Board};
use xpui_simulator::{BezelLayout, Hit, Panel, route};

/// What `MultiWindow` reports for a raw window position.
///
/// Three lines from `embedded-graphics-simulator`'s
/// `window/multi_window.rs::translate_mouse_position`: subtract the panel
/// display's offset, divide by the pixel pitch, and answer `None` when the
/// result falls off the display. The simulator itself calls the real one —
/// this exists so the routing can be exercised without opening a window.
///
/// The truncating divide is deliberate and is reproduced faithfully. It is why
/// a click a pixel above the panel comes back as panel `(0, 0)`, and why the
/// routing cannot simply believe whatever the window says.
fn what_the_window_reports(
    layout: &BezelLayout,
    panel: (i32, i32),
    scale: u32,
    at: Point,
) -> Option<Point> {
    let inset = layout.panel_offset();
    let pitch = scale as i32;
    let on_display = Point::new((at.x - inset.x) / pitch, (at.y - inset.y) / pitch);

    ((0..panel.0).contains(&on_display.x) && (0..panel.1).contains(&on_display.y))
        .then_some(on_display)
}

/// Every board that has described its body, with the window it would open.
fn bezelled() -> impl Iterator<Item = (Board, Panel, BezelLayout)> {
    Board::ALL.into_iter().filter_map(|board| {
        let panel = Panel::of(board);
        let layout = board
            .bezel
            .map(|bezel| BezelLayout::new(bezel, panel.width, panel.height, panel.scale))?;
        Some((board, panel, layout))
    })
}

/// Routes a click the way the frame loop does, without a window.
fn click(layout: &BezelLayout, panel: Panel, at: Point) -> Hit {
    let reported = what_the_window_reports(layout, (panel.width, panel.height), panel.scale, at);
    route(Some(layout), at, reported)
}

/// A spot on the shell that is neither panel nor key, stated in the device's
/// own millimetres so the premise can be checked rather than assumed.
fn bare_shell(bezel: &Bezel) -> (i32, i32) {
    (bezel.body.0 / 20, bezel.body.1 - bezel.body.1 / 40)
}

#[test]
fn a_click_in_the_middle_of_a_key_presses_it() {
    let mut checked = 0;
    for (board, panel, layout) in bezelled() {
        for button in layout.bezel().buttons {
            let at = layout.to_window(button.centre);
            assert_eq!(
                click(&layout, panel, at),
                Hit::Key(button.action),
                "{}: clicking the middle of {:?} did not press it",
                board.name,
                button.label
            );
            checked += 1;
        }
    }
    assert!(checked >= 15, "only {checked} keys were tried");
}

#[test]
fn a_click_on_bare_shell_presses_nothing() {
    for (board, panel, layout) in bezelled() {
        let bezel = layout.bezel();
        let spot = bare_shell(bezel);

        // The premise, so this cannot quietly become a click on a key.
        assert!(
            bezel.button_at(spot).is_none(),
            "{}: {spot:?} grew a key; pick another spot",
            board.name
        );
        let (x, y, width, height) = bezel.panel_rect();
        assert!(
            !((x..x + width).contains(&spot.0) && (y..y + height).contains(&spot.1)),
            "{}: {spot:?} is on the panel; pick another spot",
            board.name
        );

        assert_eq!(
            click(&layout, panel, layout.to_window(spot)),
            Hit::Body,
            "{}: the bare shell reported something to press",
            board.name
        );
    }
}

/// The one the whole task turns on: a click on the panel must arrive at the
/// *panel* coordinate, with the inset taken back off.
#[test]
fn a_click_on_the_panel_arrives_in_panel_pixels() {
    for (board, panel, layout) in bezelled() {
        let inset = layout.panel_offset();
        assert_ne!(
            inset,
            Point::ORIGIN,
            "{}: the panel is not inset at all, so this proves nothing",
            board.name
        );

        // Thirty panel pixels right of the panel's corner and forty down.
        let want = Point::new(30, 40);
        let scale = panel.scale as i32;
        let at = Point::new(inset.x + want.x * scale, inset.y + want.y * scale);

        let hit = click(&layout, panel, at);
        assert_eq!(
            hit,
            Hit::Panel(want),
            "{}: a click at window {at:?} reached the panel as {hit:?}",
            board.name
        );
        assert_ne!(
            hit,
            Hit::Panel(at),
            "{}: the window coordinate reached the screen — the inset was \
             never subtracted",
            board.name
        );
    }
}

// What that panel coordinate then *becomes* — a tap, a drag, a swipe, a
// gesture — is `tests/touch.rs`. Routing answers where a click landed and
// stops there.

/// The inset itself, checked against the body rather than against the code
/// that produced it. Without this the test above passes even when the offset
/// is wrong, because it builds its own click out of the same wrong number.
#[test]
fn the_panel_is_inset_where_the_body_says_it_is() {
    for (board, _, layout) in bezelled() {
        let origin = layout.bezel().panel_origin;
        let back = layout.to_device(layout.panel_offset());

        assert!(
            (back.0 - origin.0).abs() <= 1 && (back.1 - origin.1).abs() <= 1,
            "{}: the panel is inset at {:?}, which is {back:?} on the device — \
             the body puts it at {origin:?}",
            board.name,
            layout.panel_offset()
        );
    }
}

/// A key drawn over the screen would take clicks the firmware should have had,
/// and this is the check in window pixels rather than in millimetres — the
/// place where a wrong inset shows up.
#[test]
fn the_panel_and_the_keys_never_share_a_window_pixel() {
    for (board, _, layout) in bezelled() {
        let inset = layout.panel_offset();
        let (panel_width, panel_height) = layout.panel_size();

        for button in layout.bezel().buttons {
            let centre = layout.to_window(button.centre);
            let (width, height) = layout.to_window_size(button.size);
            let (left, top) = (centre.x - width / 2, centre.y - height / 2);

            let overlaps = left < inset.x + panel_width
                && left + width > inset.x
                && top < inset.y + panel_height
                && top + height > inset.y;
            assert!(
                !overlaps,
                "{}: {:?} is drawn over the panel",
                board.name, button.label
            );

            let (window_width, window_height) = layout.window_size();
            assert!(
                left >= 0
                    && top >= 0
                    && left + width <= window_width
                    && top + height <= window_height,
                "{}: {:?} is drawn off the window",
                board.name,
                button.label
            );
        }
    }
}

/// Every pixel of every window, and not one of them outside the panel may come
/// back as a touch.
///
/// The interesting pixels are the two or three immediately above and left of
/// the panel: the window's own translation divides and truncates towards zero,
/// so it reports those as panel `(0, 0)`. They are shell, and on a Badger a
/// touch there is a coordinate the hardware has no way of producing.
#[test]
fn a_click_off_the_panel_is_never_delivered_as_a_touch() {
    for (board, panel, layout) in bezelled() {
        let (window_width, window_height) = layout.window_size();
        let inset = layout.panel_offset();
        let (panel_width, panel_height) = layout.panel_size();
        let on_the_panel = |at: Point| {
            (inset.x..inset.x + panel_width).contains(&at.x)
                && (inset.y..inset.y + panel_height).contains(&at.y)
        };

        let (mut panels, mut keys, mut shell) = (0u32, 0u32, 0u32);
        for y in 0..window_height {
            for x in 0..window_width {
                let at = Point::new(x, y);
                match click(&layout, panel, at) {
                    Hit::Panel(_) => {
                        assert!(
                            on_the_panel(at),
                            "{}: a click at {at:?} is on the shell and was \
                             delivered as a touch",
                            board.name
                        );
                        panels += 1;
                    }
                    Hit::Key(_) => {
                        assert!(
                            !on_the_panel(at),
                            "{}: a click at {at:?} is on the panel and pressed \
                             a physical key",
                            board.name
                        );
                        keys += 1;
                    }
                    Hit::Body => shell += 1,
                }
            }
        }

        // Vacuity: a routing that answered `Body` to everything would satisfy
        // both assertions above without doing anything at all.
        assert_eq!(
            panels,
            panel_width as u32 * panel_height as u32,
            "{}: the panel took {panels} of its {} pixels",
            board.name,
            panel_width * panel_height
        );
        assert!(
            keys > 0 && shell > 0,
            "{}: {keys} key, {shell} shell",
            board.name
        );
    }
}

/// Zoom is a window concern. It changes how big the device is drawn and
/// nothing about where anything is on it.
#[test]
fn scaling_grows_the_body_without_moving_a_key() {
    let board = Board::BADGER_2040;
    let bezel = board.bezel.expect("the Badger has a bezel");

    let mut previous: Option<(i32, i32)> = None;
    for scale in 1..=4u32 {
        let layout = BezelLayout::new(bezel, board.width, board.height, scale);
        let size = layout.window_size();

        if let Some(before) = previous {
            assert!(
                size.0 > before.0 && size.1 > before.1,
                "scale {scale} drew the body at {size:?}, no bigger than {before:?}"
            );
        }
        previous = Some(size);

        let panel = Panel::of(board).scaled(scale);
        for button in bezel.buttons {
            let at = layout.to_window(button.centre);
            assert_eq!(
                click(&layout, panel, at),
                Hit::Key(button.action),
                "at {scale}x, the middle of {:?} pressed something else",
                button.label
            );
        }
        assert_eq!(
            click(&layout, panel, layout.to_window(bare_shell(&bezel))),
            Hit::Body,
            "at {scale}x, the bare shell pressed something"
        );
    }
}

/// The window has to be bigger than the panel, or there is no body to press.
#[test]
fn a_described_body_is_bigger_than_the_panel_it_holds() {
    for (board, panel, layout) in bezelled() {
        let (window_width, window_height) = layout.window_size();
        let (panel_width, panel_height) = panel.size_in_window();

        assert!(
            window_width > panel_width && window_height > panel_height,
            "{}: a {window_width}x{window_height} window round a \
             {panel_width}x{panel_height} panel",
            board.name
        );
    }
}

/// A board that has not described its body opens a window that is exactly the
/// panel, as it always did.
#[test]
fn a_board_with_no_bezel_is_all_panel() {
    // Every device described here has a body, so this is the path a board of
    // some other size takes: a window that is exactly the panel.
    for board in [
        Board::custom("plain", 480, 800, false),
        Board::custom("wide", 400, 300, true),
    ] {
        assert!(board.bezel.is_none(), "a custom board has no body");

        let panel = Panel::of(board);
        assert_eq!(
            panel.size_in_window(),
            (
                board.width * panel.scale as i32,
                board.height * panel.scale as i32
            ),
            "{}: the window is not the panel",
            board.name
        );
    }

    // With no body there is nothing to hit-test against, so whatever the
    // window says about the panel is the whole answer.
    let at = Point::new(120, 60);
    assert_eq!(route(None, at, Some(at)), Hit::Panel(at));
    assert_eq!(route(None, at, None), Hit::Body);
}

/// The X3 carries Up and Down on its side, and clicking them there has to
/// reach the firmware as those buttons.
#[test]
fn the_x3s_side_keys_are_pressable() {
    let panel = Panel::of(Board::X3);
    let bezel = Board::X3.bezel.expect("the X3 has a bezel");
    let layout = BezelLayout::new(bezel, panel.width, panel.height, panel.scale);
    let inset = layout.panel_offset();
    let (panel_width, _) = layout.panel_size();

    // One key per edge, which is what `hasEdgeSideButtons` names this board
    // for. Up is on the left, Down on the right.
    for (label, expected, on_the_right) in [
        ("Prev", Button::PageBack, false),
        ("Next", Button::PageForward, true),
    ] {
        let button = bezel
            .button_labelled(label)
            .unwrap_or_else(|| panic!("the X3 has a {label} key"));

        let at = layout.to_window(button.centre);
        if on_the_right {
            assert!(
                at.x > inset.x + panel_width,
                "{label} should sit past the panel's right edge"
            );
        } else {
            assert!(
                at.x < inset.x,
                "{label} should sit before the panel's left edge"
            );
        }
        assert_eq!(
            click(&layout, panel, at),
            Hit::Key(KeyAction::Press(expected))
        );
    }
}
