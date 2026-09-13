# Documentation

[`../README.md`](../README.md) is the front page. Every document in this
repository, and what each is for:

| | |
|---|---|
| [running.md](running.md) | the operator's guide, in the sections below |
| [Running the gallery](running.md#running-the-gallery) | `cargo run -p xpui-gallery -- --board x3`, and the seven slugs it takes |
| [Boards](running.md#boards) | `Panel::of(board)`, and `Simulator::boards(&[..])` for the cycle `B` walks |
| [Keys and mouse](running.md#keys-and-mouse) | what every key sends, and how a click becomes a tap |
| [Changing it while it runs](running.md#changing-it-while-it-runs) | board, zoom, body, screenshot — without restarting |
| [The device around the panel](running.md#the-device-around-the-panel) | the bezel drawn from millimetres, with clickable keys |
| [`--frames N`](running.md#--frames-n) | why the loop is testable, and how CI runs it |
| [The mouse as a finger](running.md#the-mouse-as-a-finger) | a drag is a swipe, the wheel is a scroll, and where the edge gestures live |
| [Headless](running.md#headless) | screenshots with no window, and no window at all |
| [When it does not run](running.md#when-it-does-not-run) | the failures people actually hit |
| [design.md](design.md) | why the window redraws when nothing changed, and the other arguments behind choices the code states in one sentence |
| [contributing.md](contributing.md) | SDL2 first, then building it, the gate, the five review steps, and how a commit is written |
