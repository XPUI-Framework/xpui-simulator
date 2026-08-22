//! CrossPoint's touch model, as the simulator implements it.
//!
//! Every case here is a rule the firmware already keeps, and the reason this
//! file exists is that a mouse cannot check any of them by hand: the
//! difference between a 59-pixel roll and a 60-pixel flick is one frame of one
//! gesture, and the eye cannot tell which one it just made.
//!
//! The classifier is driven by `(position, timestamp)` alone, so none of this
//! opens a window or needs SDL.

use xpui::{Point, SwipeDir};
use xpui_boards_core::Board;
use xpui_boards_pimoroni as pimoroni;
use xpui_simulator::{EdgeGesture, Touch, Touchscreen};

mod devices;

/// A touch panel 480x800, which makes the edge bands 120 px at the sides and
/// 112 px top and bottom.
///
/// Deliberately not one of the presets: the model reads a width, a height and
/// a touch flag, and pinning the thresholds to a device's current dimensions
/// would rewrite these tests every time a board is re-measured.
fn touchscreen() -> Touchscreen {
    Touchscreen::for_board(Board::custom("test", PANEL.0, PANEL.1, true))
}

const PANEL: (i32, i32) = (480, 800);

/// A whole contact: down, samples along the way, then up `over_ms` later.
///
/// The intermediate samples matter. A mouse reports the journey, not only the
/// endpoints, and the slop latches are fed by every sample — a drag that went
/// out and came back is not the same contact as one that never left.
fn drag(screen: &mut Touchscreen, from: Point, to: Point, over_ms: u32) -> Vec<Touch> {
    const STEPS: i32 = 4;

    screen.down(from, 0);
    for step in 1..=STEPS {
        screen.moved(Point::new(
            from.x + (to.x - from.x) * step / STEPS,
            from.y + (to.y - from.y) * step / STEPS,
        ));
    }
    screen.up(Some(to), over_ms).into_iter().collect()
}

/// The same, moving `by` pixels horizontally from the middle of the panel.
fn drag_across(screen: &mut Touchscreen, by: i32, over_ms: u32) -> Vec<Touch> {
    let from = Point::new(PANEL.0 / 2, PANEL.1 / 2);
    drag(screen, from, Point::new(from.x + by, from.y), over_ms)
}

fn tapped(events: &[Touch]) -> Option<Point> {
    events.iter().find_map(|event| match event {
        Touch::Tap(at) => Some(*at),
        _ => None,
    })
}

fn swiped(events: &[Touch]) -> Option<SwipeDir> {
    events.iter().find_map(|event| match event {
        Touch::Swipe(direction) => Some(*direction),
        _ => None,
    })
}

fn edge(events: &[Touch]) -> Option<EdgeGesture> {
    events.iter().find_map(|event| match event {
        Touch::Edge(gesture) => Some(*gesture),
        _ => None,
    })
}

// -- taps -----------------------------------------------------------------

/// And at the position the finger went **down**, not where it came up. The
/// centroid drifts 10-20 px as a finger rolls off during a lift, and routing
/// the release point makes small targets feel unreliable.
#[test]
fn a_small_drag_is_a_tap_at_the_point_the_finger_landed() {
    let mut screen = touchscreen();
    let down = Point::new(200, 400);
    let up = Point::new(210, 400);

    let events = drag(&mut screen, down, up, 60);

    assert_eq!(
        events,
        vec![Touch::Released, Touch::Tap(down)],
        "ten pixels of drift is a tap, and nothing else"
    );
    assert_ne!(
        tapped(&events),
        Some(up),
        "the tap was routed to where the finger let go, not where it touched"
    );
}

/// **The dead-band regression.** 45 px is past the 28 px stationary slop that
/// cancels a hold, and short of the 60 px that makes a swipe. Gating the
/// release on the stationary slop — which CrossPoint's own simulator still
/// does — leaves a 29..59 px band where an ordinary finger roll is neither a
/// tap nor a swipe, and the control under it simply never fires.
#[test]
fn a_forty_five_pixel_roll_is_still_a_tap() {
    let mut screen = touchscreen();
    let from = Point::new(200, 400);

    let events = drag(&mut screen, from, Point::new(from.x + 45, from.y), 120);

    assert_eq!(tapped(&events), Some(from), "45 px of roll ate the tap");
    assert_eq!(swiped(&events), None, "45 px is short of a swipe");
}

/// The two thresholds meet exactly, so every contact is one or the other and
/// none is neither. Sweeping the distance is what makes this a *relationship*
/// rather than two numbers that happen to be right today: move either
/// threshold and some distance falls in the gap between them.
#[test]
fn no_dead_band_between_a_tap_and_a_swipe() {
    for distance in 0..=90 {
        let mut screen = touchscreen();
        let events = drag_across(&mut screen, distance, 200);
        let (tap, swipe) = (tapped(&events).is_some(), swiped(&events).is_some());

        assert!(
            tap != swipe,
            "{distance} px was {}: {events:?}",
            if tap {
                "both a tap and a swipe"
            } else {
                "neither a tap nor a swipe"
            }
        );
        assert_eq!(tap, distance <= 59, "at {distance} px the tap is wrong");
        assert_eq!(swipe, distance >= 60, "at {distance} px the swipe is wrong");
    }
}

/// Every contact ends in a release, whatever else it was — a screen that
/// closes a drag on the release edge must not depend on what the drag turned
/// out to be.
#[test]
fn every_contact_ends_in_a_release() {
    let mut screen = touchscreen();
    for (distance, over_ms) in [(0, 50), (45, 200), (70, 200), (70, 900)] {
        let events = drag_across(&mut screen, distance, over_ms);
        assert_eq!(
            events.first(),
            Some(&Touch::Released),
            "{distance} px over {over_ms} ms ended without a release: {events:?}"
        );
    }
}

// -- swipes ---------------------------------------------------------------

#[test]
fn a_seventy_pixel_flick_is_a_swipe_the_way_it_travelled() {
    let mut screen = touchscreen();
    assert_eq!(
        swiped(&drag_across(&mut screen, 70, 200)),
        Some(SwipeDir::Right)
    );

    let mut screen = touchscreen();
    assert_eq!(
        swiped(&drag_across(&mut screen, -70, 200)),
        Some(SwipeDir::Left)
    );

    let mut screen = touchscreen();
    let from = Point::new(240, 400);
    let up = drag(&mut screen, from, Point::new(from.x, from.y - 70), 200);
    assert_eq!(
        swiped(&up),
        Some(SwipeDir::Up),
        "a swipe towards the top is Up"
    );
}

/// A swipe is a *flick*: the same travel taken slowly is a drag that happens
/// to have ended somewhere else, and reporting it as a swipe turns a careful
/// reposition into a page turn.
#[test]
fn the_same_flick_taken_too_slowly_is_not_a_swipe() {
    let mut screen = touchscreen();
    let inside = drag_across(&mut screen, 70, 700);
    assert_eq!(
        swiped(&inside),
        Some(SwipeDir::Right),
        "700 ms is inside the window"
    );

    let mut screen = touchscreen();
    let outside = drag_across(&mut screen, 70, 900);
    assert_eq!(swiped(&outside), None, "900 ms is a drag, not a flick");
    assert_eq!(
        tapped(&outside),
        None,
        "and it travelled too far to be a tap"
    );
}

/// 60 px on *either* axis qualifies. A mostly-horizontal swipe must not be
/// disqualified for having barely moved vertically, which is what an `and`
/// would do.
#[test]
fn one_axis_is_enough_to_qualify() {
    let mut screen = touchscreen();
    let from = Point::new(240, 400);

    let events = drag(&mut screen, from, Point::new(from.x + 60, from.y + 3), 200);

    assert_eq!(swiped(&events), Some(SwipeDir::Right));
}

#[test]
fn a_diagonal_resolves_to_its_dominant_axis_and_a_tie_goes_horizontal() {
    let from = Point::new(240, 400);
    let cases = [
        ((70, 40), SwipeDir::Right),
        ((-70, 40), SwipeDir::Left),
        ((40, 70), SwipeDir::Down),
        ((40, -70), SwipeDir::Up),
        ((70, 70), SwipeDir::Right),
        ((-70, -70), SwipeDir::Left),
    ];

    for ((dx, dy), want) in cases {
        let mut screen = touchscreen();
        let events = drag(&mut screen, from, Point::new(from.x + dx, from.y + dy), 200);
        assert_eq!(
            swiped(&events),
            Some(want),
            "({dx}, {dy}) resolved the wrong way"
        );
    }
}

// -- the long press -------------------------------------------------------

/// It fires while the finger is still down — that is the whole point of it —
/// and once per contact.
#[test]
fn holding_still_fires_a_long_press_while_the_finger_is_down() {
    let mut screen = touchscreen();
    let down = Point::new(200, 400);
    screen.down(down, 1_000);

    assert!(
        screen.tick(1_499).is_empty(),
        "499 ms is not a long press yet"
    );
    assert_eq!(
        screen.tick(1_500).into_iter().collect::<Vec<_>>(),
        vec![Touch::LongPress(down)],
        "500 ms of stillness is a long press, at the point of contact"
    );
    assert!(
        screen.tick(1_600).is_empty(),
        "it fired twice for one contact"
    );
    assert!(screen.is_down(), "the finger is still down");
}

/// Motion past the *stationary* slop cancels it. This is the threshold whose
/// only job is this one, and it is 28 px rather than the release slop's 59.
#[test]
fn motion_past_the_stationary_slop_cancels_the_long_press() {
    let mut screen = touchscreen();
    let down = Point::new(200, 400);

    screen.down(down, 0);
    screen.moved(Point::new(down.x + 30, down.y));
    assert!(
        screen.tick(500).is_empty(),
        "30 px of travel was not a hold"
    );

    // And the boundary, so the slop cannot quietly become 30 or 60.
    let mut screen = touchscreen();
    screen.down(down, 0);
    screen.moved(Point::new(down.x + 28, down.y));
    assert_eq!(
        screen.tick(500).into_iter().collect::<Vec<_>>(),
        vec![Touch::LongPress(down)],
        "28 px of wobble is still holding still"
    );
}

/// Coming back does not undo having left: the latch is over the whole contact,
/// because a finger that wandered out and returned was never holding still.
#[test]
fn a_finger_that_wandered_and_came_back_was_not_holding_still() {
    let mut screen = touchscreen();
    let down = Point::new(200, 400);

    screen.down(down, 0);
    screen.moved(Point::new(down.x + 40, down.y));
    screen.moved(down);

    assert!(screen.tick(500).is_empty());
}

// -- edge gestures --------------------------------------------------------

/// Back is a left-to-right swipe that *started* in the left quarter. Anchoring
/// it there is what leaves mid-screen horizontal swipes to the screens that
/// consume them — a reader paging with a right swipe would otherwise navigate
/// back instead of turning the page.
#[test]
fn a_right_swipe_from_the_left_band_is_back_and_from_mid_screen_is_not() {
    let mut screen = touchscreen();
    let from_edge = drag(&mut screen, Point::new(10, 400), Point::new(80, 400), 200);
    assert_eq!(edge(&from_edge), Some(EdgeGesture::Back));
    assert_eq!(
        swiped(&from_edge),
        Some(SwipeDir::Right),
        "the swipe is still reported; the edge meaning rides along with it"
    );

    let mut screen = touchscreen();
    let from_middle = drag(&mut screen, Point::new(240, 400), Point::new(310, 400), 200);
    assert_eq!(
        edge(&from_middle),
        None,
        "a mid-screen swipe navigated back — the band is not anchored"
    );
    assert_eq!(swiped(&from_middle), Some(SwipeDir::Right));
}

/// The band is a quarter of the panel, inclusive, and the arithmetic that
/// says so truncates the way the firmware's does.
#[test]
fn the_side_band_is_a_quarter_of_the_panel() {
    let band = PANEL.0 / 4;

    for (start, wanted) in [(band, Some(EdgeGesture::Back)), (band + 1, None)] {
        let mut screen = touchscreen();
        let events = drag(
            &mut screen,
            Point::new(start, 400),
            Point::new(start + 70, 400),
            200,
        );
        assert_eq!(edge(&events), wanted, "a swipe from x = {start}");
    }
}

#[test]
fn an_up_swipe_from_the_bottom_band_is_home() {
    let band = PANEL.1 * 14 / 100;
    let inside = PANEL.1 - band;

    let mut screen = touchscreen();
    let events = drag(
        &mut screen,
        Point::new(240, inside),
        Point::new(240, inside - 70),
        200,
    );
    assert_eq!(edge(&events), Some(EdgeGesture::Home));
    assert_eq!(swiped(&events), Some(SwipeDir::Up));

    let mut screen = touchscreen();
    let above = drag(
        &mut screen,
        Point::new(240, inside - 1),
        Point::new(240, inside - 71),
        200,
    );
    assert_eq!(edge(&above), None, "one pixel above the band is not Home");
}

#[test]
fn a_down_swipe_from_the_top_band_is_the_menu_gesture() {
    let band = PANEL.1 * 14 / 100;

    let mut screen = touchscreen();
    let events = drag(
        &mut screen,
        Point::new(240, band),
        Point::new(240, band + 70),
        200,
    );
    assert_eq!(edge(&events), Some(EdgeGesture::Menu));

    let mut screen = touchscreen();
    let below = drag(
        &mut screen,
        Point::new(240, band + 1),
        Point::new(240, band + 71),
        200,
    );
    assert_eq!(
        edge(&below),
        None,
        "one pixel below the band is not the menu"
    );
}

/// Each edge needs its own axis *strictly* dominant, so a perfect diagonal out
/// of a corner is neither gesture. A swipe that could equally be two things
/// must not be allowed to be either.
#[test]
fn a_perfect_diagonal_from_a_corner_is_no_edge_gesture_at_all() {
    let mut screen = touchscreen();
    let events = drag(&mut screen, Point::new(10, 10), Point::new(80, 80), 200);

    assert_eq!(edge(&events), None);
    assert_eq!(
        swiped(&events),
        Some(SwipeDir::Right),
        "it is still a swipe — the tie only settles the direction"
    );
}

// -- boards with no touchscreen -------------------------------------------

/// A device with no touchscreen cannot produce a touch, so the simulator must
/// not invent one. Otherwise a screen ships depending on a tap the hardware
/// will never send.
#[test]
fn a_board_with_no_touchscreen_reports_nothing_at_all() {
    // The premise, checked where it is written down. If the Badger ever grows
    // a touchscreen this stops compiling, rather than passing vacuously.
    const _: () = assert!(!pimoroni::BADGER_2040.touch);

    let mut badger = Touchscreen::for_board(pimoroni::BADGER_2040);

    let down = Point::new(100, 60);
    assert!(badger.down(down, 0).is_empty(), "a click became a touch");
    assert!(
        !badger.is_down(),
        "a contact opened on a board that has none"
    );
    assert!(
        badger.moved(Point::new(160, 60)).is_empty(),
        "a drag became one"
    );
    assert!(badger.tick(600).is_empty(), "a hold became a long press");
    assert!(
        badger.up(Some(Point::new(160, 60)), 200).is_empty(),
        "a release became a tap or a swipe"
    );
}

/// And every other board that has said the same thing about itself.
#[test]
fn no_board_without_touch_classifies_anything() {
    for board in devices::ALL.into_iter().filter(|board| !board.touch) {
        let mut screen = Touchscreen::for_board(board);
        let middle = Point::new(board.width / 2, board.height / 2);

        let events = drag(
            &mut screen,
            middle,
            Point::new(middle.x + 70, middle.y),
            200,
        );

        assert!(events.is_empty(), "{}: {events:?}", board.name);
    }
}

/// The positive control for the two tests above: the same clicks on a board
/// that *does* have a touchscreen are a touch. Without this they would both
/// still pass with the classifier deleted.
#[test]
fn a_board_with_a_touchscreen_reports_everything() {
    for board in devices::ALL.into_iter().filter(|board| board.touch) {
        let mut screen = Touchscreen::for_board(board);
        let middle = Point::new(board.width / 2, board.height / 2);

        let events = drag(
            &mut screen,
            middle,
            Point::new(middle.x + 70, middle.y),
            200,
        );

        assert_eq!(
            swiped(&events),
            Some(SwipeDir::Right),
            "{}: a flick across a touchscreen reported {events:?}",
            board.name
        );
    }
}

// -- the ends of a contact ------------------------------------------------

/// A mouse released outside the panel still ends the contact. A finger that
/// left the glass has no coordinate to have lifted at, so the release is not a
/// sample — but it is still a release, and a contact left open would hold the
/// next click's classification hostage.
#[test]
fn a_release_off_the_panel_still_ends_the_contact() {
    let mut screen = touchscreen();
    let down = Point::new(200, 400);

    screen.down(down, 0);
    screen.moved(Point::new(down.x + 10, down.y));
    let events: Vec<_> = screen.up(None, 100).into_iter().collect();

    assert_eq!(events, vec![Touch::Released, Touch::Tap(down)]);
    assert!(!screen.is_down(), "the contact outlived the release");
}

/// Events with no contact behind them are ignored rather than opening one.
#[test]
fn a_move_or_a_release_with_no_contact_reports_nothing() {
    let mut screen = touchscreen();

    assert!(screen.moved(Point::new(200, 400)).is_empty());
    assert!(screen.tick(1_000).is_empty());
    assert!(screen.up(Some(Point::new(200, 400)), 100).is_empty());
}
