//! The screenshot key, without a window.
//!
//! What it must produce is a file, of this panel, that does not overwrite one
//! already there. Each of those has its own way of going wrong: a writer that
//! copies nothing still produces a plausible blank panel, and a name that
//! counts from one every run quietly replaces the shot you took last time.
//!
//! The host is process-wide, so these run one at a time.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use xpui::{App, Divider, Screen, View, vstack};
use xpui_simulator::{Board, Panel, Session, capture_panel};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A screen with ink on it, so a capture that copied nothing is a failure
/// rather than a plausible blank panel.
struct Rule;

impl Screen for Rule {
    type Message = ();

    fn body(&self) -> impl View<()> {
        vstack![8; Divider::new()]
    }

    fn update(&mut self, _message: ()) {}
}

/// An empty directory inside the workspace's `target/`.
///
/// `CARGO_TARGET_TMPDIR` rather than the working directory: Cargo runs an
/// integration test from its own crate root, which in a workspace is not where
/// the shared `target/` is.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A 1-bit BMP's width, height, and how many of its pixels are ink.
///
/// Index 0 is ink, so a cleared bit is a dark pixel. The padding at the end of
/// each row is written as paper, which is why counting cleared bits cannot be
/// fooled by it.
fn read_bmp(path: &Path) -> (i32, i32, usize) {
    let bytes = fs::read(path).expect("the screenshot was written");
    assert_eq!(&bytes[..2], b"BM", "not a BMP at all");

    let word = |at: usize| i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let data = word(10) as usize;
    let ink = bytes[data..]
        .iter()
        .map(|byte| byte.count_zeros() as usize)
        .sum();
    (word(18), word(22), ink)
}

#[test]
fn a_press_writes_this_panel() {
    let _guard = serial();
    let dir = scratch("screenshot-writes");

    let session = Session::new(Panel::of(Board::BADGER_2040));
    App::new(Rule).render();
    let path = capture_panel(session.backend(), session.board().slug, &dir);

    assert!(
        path.starts_with(&dir),
        "it went where it was told: {path:?}"
    );
    let name = path.file_name().unwrap().to_string_lossy();
    assert!(
        name.starts_with(Board::BADGER_2040.slug),
        "a file called {name} does not say which board it is of"
    );

    let (width, height, ink) = read_bmp(&path);
    assert_eq!(
        (width, height),
        (Board::BADGER_2040.width, Board::BADGER_2040.height),
        "the file is the panel — not the window, and not the device around it"
    );
    assert!(
        ink > 0 && ink < (width * height) as usize,
        "{ink} ink pixels out of {}: the capture copied nothing, or everything",
        width * height
    );
}

/// Pressing it twice gives two files, and a name already on disk is skipped —
/// so the run before this one keeps its shots too.
#[test]
fn it_never_writes_over_a_shot_that_is_already_there() {
    let _guard = serial();
    let dir = scratch("screenshot-numbering");
    let slug = Board::X4.slug;

    // As if an earlier run had left one behind.
    let taken = dir.join(format!("{slug}-1.bmp"));
    fs::write(&taken, b"not a screenshot").expect("can seed the directory");

    let session = Session::new(Panel::of(Board::X4));
    App::new(Rule).render();
    let first = capture_panel(session.backend(), session.board().slug, &dir);
    let second = capture_panel(session.backend(), session.board().slug, &dir);

    assert_ne!(first, second, "two presses, two files");
    assert_eq!(
        fs::read(&taken).expect("still there").len(),
        b"not a screenshot".len(),
        "the file that was already there was written over"
    );
    assert_eq!(
        fs::read_dir(&dir).unwrap().count(),
        3,
        "one seeded and two captured"
    );
}
