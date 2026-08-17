//! The device around the panel, drawn from primitives.
//!
//! No assets. The same reason the chrome backend draws its icons from lines
//! and rectangles applies here for a different one: manufacturer photography
//! is not licensed for reuse, and nothing public covers the rest of these
//! devices. A body from the published dimensions is the shipped path, and
//! `Bezel::artwork` is the slot where a licensed image can replace it later
//! without the layout being redesigned around it.

use embedded_graphics::mono_font::{MonoFont, MonoTextStyle, ascii};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use embedded_graphics_simulator::SimulatorDisplay;
use xpui::Button;
use xpui_boards::PhysicalButton;

use crate::layout::BezelLayout;

/// What is behind the device: visible only where the body's corners round off.
const DESK: Rgb888 = Rgb888::new(0x14, 0x16, 0x18);
/// The body itself.
const SHELL: Rgb888 = Rgb888::new(0x3A, 0x3E, 0x44);
/// The well the panel is set into, darker than the shell it is cut out of so
/// the panel reads as recessed rather than pasted on.
const WELL: Rgb888 = Rgb888::new(0x1E, 0x21, 0x25);
/// A key at rest, and the same key under a finger.
const KEY: Rgb888 = Rgb888::new(0x6B, 0x71, 0x7A);
const KEY_HELD: Rgb888 = Rgb888::new(0xC8, 0xCE, 0xD6);
/// A label printed on a key, and one printed on the shell beside it.
const LEGEND_ON_KEY: Rgb888 = Rgb888::new(0x11, 0x13, 0x16);
const LEGEND_ON_SHELL: Rgb888 = Rgb888::new(0xA8, 0xAE, 0xB6);

/// Legend faces, largest first.
///
/// `embedded-graphics` ships fixed sizes rather than a scalable face, so the
/// legend picks the biggest one that still fits inside the key. A Badger key at
/// 3x is 119 pixels across, and 6x10 on it reads as a speck.
const FACES: [&MonoFont<'static>; 3] = [&ascii::FONT_10X20, &ascii::FONT_9X15, &ascii::FONT_6X10];

/// Corner radii, in tenths of a millimetre, so they scale with everything else.
const SHELL_RADIUS: i32 = 30;
const KEY_RADIUS: i32 = 12;
/// How far the well extends past the panel on each side.
const WELL_LIP: i32 = 12;

/// Repaints the whole body, with `held` — if any — pressed in.
///
/// Cheap enough to redraw whole: it is `Rgb888` on the host, and it changes
/// only when a button goes down or comes up.
pub fn paint(display: &mut SimulatorDisplay<Rgb888>, layout: &BezelLayout, held: Option<Button>) {
    let (width, height) = layout.window_size();
    let _ = display.clear(DESK);

    fill_rounded(
        display,
        rect(Point::zero(), (width, height)),
        layout.to_window_size((SHELL_RADIUS, SHELL_RADIUS)).0,
        SHELL,
    );

    well(display, layout);

    for button in layout.bezel().buttons {
        key(display, layout, button, held == Some(button.button));
    }
}

/// The recess the panel sits in: the panel's rectangle, grown by a lip.
fn well(display: &mut SimulatorDisplay<Rgb888>, layout: &BezelLayout) {
    let (lip_x, lip_y) = layout.to_window_size((WELL_LIP, WELL_LIP));
    let origin = layout.panel_offset();
    let (panel_width, panel_height) = layout.panel_size();

    fill_rounded(
        display,
        rect(
            Point::new(origin.x - lip_x, origin.y - lip_y),
            (panel_width + lip_x * 2, panel_height + lip_y * 2),
        ),
        lip_x,
        WELL,
    );
}

fn key(
    display: &mut SimulatorDisplay<Rgb888>,
    layout: &BezelLayout,
    button: &PhysicalButton,
    held: bool,
) {
    let centre = layout.to_window(button.centre);
    let (width, height) = layout.to_window_size(button.size);
    let face = rect(
        Point::new(centre.x - width / 2, centre.y - height / 2),
        (width, height),
    );

    fill_rounded(
        display,
        face,
        layout.to_window_size((KEY_RADIUS, KEY_RADIUS)).0,
        if held { KEY_HELD } else { KEY },
    );
    legend(display, button.label, face);
}

/// The label, printed on the key when it fits and under it when it does not.
///
/// Under it rather than nowhere: a key too small for its own name is still a
/// key you have to be able to name.
fn legend(display: &mut SimulatorDisplay<Rgb888>, label: &str, face: Rectangle) {
    if label.is_empty() {
        return;
    }

    // A margin either side, so a legend does not run into the key's rounding.
    const MARGIN: u32 = 4;
    let letters = label.chars().count() as u32;
    let width =
        |font: &MonoFont<'_>| letters * (font.character_size.width + font.character_spacing);

    let on_the_key = FACES.into_iter().find(|font| {
        width(font) + MARGIN <= face.size.width
            && font.character_size.height + MARGIN <= face.size.height
    });

    let middle = face.top_left.x + face.size.width as i32 / 2;
    let bottom = face.top_left.y + face.size.height as i32;
    let (font, y, baseline, colour) = match on_the_key {
        Some(font) => (
            font,
            face.top_left.y + face.size.height as i32 / 2,
            Baseline::Middle,
            LEGEND_ON_KEY,
        ),
        None => (
            FACES[FACES.len() - 1],
            bottom + 3,
            Baseline::Top,
            LEGEND_ON_SHELL,
        ),
    };

    let placement = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(baseline)
        .build();
    let _ = Text::with_text_style(
        label,
        Point::new(middle, y),
        MonoTextStyle::new(font, colour),
        placement,
    )
    .draw(display);
}

fn rect(top_left: Point, size: (i32, i32)) -> Rectangle {
    Rectangle::new(
        top_left,
        Size::new(size.0.max(0) as u32, size.1.max(0) as u32),
    )
}

/// A filled rounded rectangle. The radius is confined to the rectangle by
/// `embedded-graphics` itself, so a body smaller than its own corners still
/// draws.
fn fill_rounded(
    display: &mut SimulatorDisplay<Rgb888>,
    area: Rectangle,
    radius: i32,
    colour: Rgb888,
) {
    let radius = radius.max(0) as u32;
    let _ = RoundedRectangle::with_equal_corners(area, Size::new(radius, radius))
        .into_styled(PrimitiveStyle::with_fill(colour))
        .draw(display);
}
