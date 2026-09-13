# Design decisions

The arguments behind choices the code states in one sentence. Each section
names the file or item that carries the sentence; what the window does with
them is [running.md](running.md).

## The window redraws when nothing changed (`src/run.rs`)

E-ink takes a second or more to refresh, so `App` only repaints when something
actually changed. The window still has to be pumped every frame or the OS
thinks the app has hung — so the loop pushes the unchanged framebuffer and
sleeps, rather than spinning a core.

## Touch is classified here, never in the framework (`src/touch/`)

`xpui` is handed taps, drags, swipes and gestures, and where those came from
is the host's business. The model is CrossPoint's, ported rule for rule and
constant for constant, because a simulator that classifies differently agrees
with the device right up to where it matters: the 45-pixel finger roll one
slop calls nothing and the device calls a tap. The constants, and why the two
slops differ, are in [running.md](running.md). Positions are panel pixels,
the space a screen is laid out in; a window's own pixels would put every
threshold somewhere else at every zoom.

## The window never resizes (`Session`)

`MultiWindow` fixes its SDL window and its streaming texture in the
constructor, and there is no resize API. So the window is opened once, big
enough for the largest board any key can reach, and every smaller one is
letterboxed into the middle of it. Everything that follows — how far zoom may
go, where the device sits, which backend is installed — is decided in one
place, so the painter and the event loop measure against one set of numbers.

## Presses arrive raw (`Keys`)

Hardware sends presses; what two of them close together *mean* is the
firmware's decision, and a simulator that decides for it is standing in for
something the hardware does not do. So the presses arrive raw, and a caller
that wants to read something into them says so through `Keys`: a translation
renames a key that went down, an injection is a press produced by a timer.
The gallery's double-press-for-Back is the worked example.
