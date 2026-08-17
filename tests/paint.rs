//! That the body is actually drawn.
//!
//! The routing tests prove a click on a key reaches the firmware. They would
//! all still pass if the body were never painted at all — the window would be
//! blank and every key invisible, and nothing would say so.

use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;

/// The two crates each have a `Point`; the layout speaks the framework's and
/// the display speaks embedded-graphics'.
fn eg(at: xpui::Point) -> Point {
    Point::new(at.x, at.y)
}
use embedded_graphics_simulator::SimulatorDisplay;
use xpui_simulator::{BezelLayout, Board, Panel, paint_body};

/// Renders a board's body, and reports how many pixels differ from the
/// background it started as.
fn painted(board: Board) -> (SimulatorDisplay<Rgb888>, BezelLayout) {
    let bezel = board.bezel.expect("this board has a body");
    let panel = Panel::of(board);
    let layout = BezelLayout::new(bezel, panel.width, panel.height, panel.scale);
    let (width, height) = layout.window_size();

    let mut display = SimulatorDisplay::new(Size::new(width as u32, height as u32));
    paint_body(&mut display, &layout, None);
    (display, layout)
}

fn distinct_colours(
    display: &SimulatorDisplay<Rgb888>,
    at: xpui::Point,
    size: (i32, i32),
) -> usize {
    let mut seen: Vec<Rgb888> = Vec::new();
    for y in at.y..at.y + size.1 {
        for x in at.x..at.x + size.0 {
            let colour = display.get_pixel(Point::new(x, y));
            if !seen.contains(&colour) {
                seen.push(colour);
            }
        }
    }
    seen.len()
}

/// Every key has to be visible, or you cannot press what you cannot see.
#[test]
fn every_key_is_drawn() {
    for board in Board::ALL {
        let Some(bezel) = board.bezel else { continue };
        let (display, layout) = painted(board);

        for button in bezel.buttons {
            let face = layout.to_window((
                button.centre.0 - button.size.0 / 2,
                button.centre.1 - button.size.1 / 2,
            ));
            let size = layout.to_window_size(button.size);

            assert!(
                distinct_colours(&display, face, size) > 1,
                "{}: {:?} is a flat block — the key is not drawn, or its \
                 legend is missing",
                board.name,
                button.label
            );
        }
    }
}

/// The well the panel sits in must be distinguishable from the body around it,
/// or the screen has no edge and the device reads as a slab.
#[test]
fn the_panel_sits_in_a_visible_well() {
    for board in Board::ALL {
        let Some(_) = board.bezel else { continue };
        let (display, layout) = painted(board);

        let inset = layout.panel_offset();
        let body = display.get_pixel(Point::new(2, 2));
        let well = display.get_pixel(Point::new(inset.x - 1, inset.y - 1));

        assert_ne!(
            body, well,
            "{}: the panel's surround is the same colour as the body",
            board.name
        );
    }
}

/// A held key has to look held, or pressing one gives no feedback at all.
#[test]
fn a_held_key_looks_different() {
    for board in Board::ALL {
        let Some(bezel) = board.bezel else { continue };
        let button = bezel.buttons[0];
        let panel = Panel::of(board);
        let layout = BezelLayout::new(bezel, panel.width, panel.height, panel.scale);
        let (width, height) = layout.window_size();

        let mut idle = SimulatorDisplay::new(Size::new(width as u32, height as u32));
        paint_body(&mut idle, &layout, None);

        let mut held = SimulatorDisplay::new(Size::new(width as u32, height as u32));
        paint_body(&mut held, &layout, Some(button.button));

        let centre = layout.to_window(button.centre);
        assert_ne!(
            idle.get_pixel(eg(centre)),
            held.get_pixel(eg(centre)),
            "{}: holding {:?} changes nothing on the key",
            board.name,
            button.label
        );
    }
}
