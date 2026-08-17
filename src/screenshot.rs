//! Writing the panel to a file you can open.
//!
//! The panel only, never the window: the body around it is this crate's
//! drawing, and a picture of a screen with a simulated device in it is not a
//! picture of what the device would show. What lands in the file is what the
//! firmware put in the framebuffer.

use std::path::{Path, PathBuf};

use embedded_graphics::prelude::*;
use xpui_eg::{Backend, Framebuffer};

use crate::panel::PanelDisplay;

/// Writes the panel to `dir` and answers where it went.
///
/// Named `<slug>-<n>.bmp`, with `n` the first number not already taken, so
/// pressing the key twice gives two files and a second session does not
/// overwrite the first one's.
pub fn capture(backend: &Backend<PanelDisplay>, slug: &str, dir: &Path) -> PathBuf {
    let frame = backend.with_display(copy_out);
    frame.write_bmp_in(dir.to_path_buf(), &next_name(dir, slug))
}

/// The panel's pixels, in the shape the BMP writer takes.
///
/// A copy rather than a borrow because the two are different types: the window
/// draws into a `SimulatorDisplay` and the writer lives on [`Framebuffer`].
/// Both are a pixel per entry and the panel is at most half a megapixel, so
/// the copy costs less than a frame of the loop it runs in.
fn copy_out(display: &mut PanelDisplay) -> Framebuffer {
    let size = display.bounding_box().size;
    let mut frame = Framebuffer::new(size.width as i32, size.height as i32);

    for y in 0..frame.height {
        for x in 0..frame.width {
            // `is_on` is ink, because the simulator installs `INK_IS_ON`. A
            // backend built with the other polarity would come out inverted,
            // and this is the one line that would have to know.
            frame.pixels[(y * frame.width + x) as usize] =
                display.get_pixel(Point::new(x, y)).is_on();
        }
    }
    frame
}

/// The first `<slug>-<n>` nobody has used.
fn next_name(dir: &Path, slug: &str) -> String {
    for n in 1.. {
        let name = format!("{slug}-{n}");
        if !dir.join(format!("{name}.bmp")).exists() {
            return name;
        }
    }
    unreachable!("the range is unbounded")
}

/// Where screenshots go: `$XPUI_SCREENSHOT_DIR`, or `target/screenshots`.
///
/// The same rule the framebuffer's own writer follows, so a screenshot taken
/// from the window and one taken from a test land in the same place.
pub fn directory() -> PathBuf {
    xpui_eg::framebuffer::screenshot_dir()
}
