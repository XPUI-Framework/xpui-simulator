//! Real devices, for the tests only.
//!
//! This crate names no device — that is the point of it, and
//! `crates/backend/simulator/src/` is greped to prove it. Its *tests* need
//! bodies with keys on them to be worth anything: a bezel with nothing in it
//! cannot show that a click lands on the key under it, and a cycle of one
//! cannot show that `B` walks the caller's order.
//!
//! So the vendor crates are dev-dependencies, and this is the list they
//! compose to. It is deliberately the same seven the gallery ships, in the
//! same order, so a failure here and a failure there name the same board.

use xpui_simulator::Board;

/// The seven, in vendor order — the same list and order as `gallery::boards`.
///
/// Two hand-written arrays in two crates that cannot see each other: the
/// gallery depends on the simulator, so nothing here can reach it. What both
/// *can* see is the vendor crates, so both assert the same thing against them
/// and a board added to a vendor stops both building until somebody looks.
pub const ALL: [Board; 7] = [
    xpui_boards_xteink::X3,
    xpui_boards_xteink::X4,
    xpui_boards_xteink::X4_PRO,
    xpui_boards_seeed::STICKY,
    xpui_boards_pimoroni::BADGER_2040,
    xpui_boards_pimoroni::TUFTY_2040,
    xpui_boards_pimoroni::INKY_FRAME,
];

const _: () = assert!(
    ALL.len()
        == xpui_boards_xteink::ALL.len()
            + xpui_boards_seeed::ALL.len()
            + xpui_boards_pimoroni::ALL.len(),
    "a vendor gained or lost a board and this list did not follow"
);
