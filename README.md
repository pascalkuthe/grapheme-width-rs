# grapheme-width

This crate provides a function to compute the number of columns occupied by a single [unicode grapheme](https://unicode.org/reports/tr29/). It's distinguishing factors are that it always returns the correct **display width**,
**backwards compatability** with older unicode versions and being **lightweight**.

There are two other options that provide similar functionality already in the rust ecosystem:
* The [unicode width](https://github.com/unicode-rs/unicode-width) crate
* The [`grapheme_column_width`](https://docs.rs/termwiz/0.20.0/termwiz/cell/fn.grapheme_column_width.html) function of the [`termwiz`](https://crates.io/crates/termwiz) crate.

Both options have drawbacks

The [unicode-width](https://github.com/unicode-rs/unicode-width) crate currently doesn't support doesn't account for emoji presentation/width changes caused by emoji variation selectors VS15 and VS16 introduced in unicode 14. For example the following emojis have text presentation by default but because they are followed by VS16 they are switched to emoji presentation and double width: ✔️ 🖋️.This different width definition is not supported by all emulators yet. This crate works around that by allowing applications/users to configure the unicode support level manually.

To handle the cases described above the [`termwiz`](https://crates.io/crates/termwiz) crate (part of the [`wezterm`](https://github.com/wez/wezterm) emulator) switch to a custom grapheme width calculation based on [widecharwidth](https://github.com/ridiculousfish/widecharwidth/). To account for unicode 14 presentation changes handling for emoji variations were also added. However, the termwiz crate is a very heavy dependency. Not only does it contain a LOT of functionality itself it also has a large number of (transitive) dependencies. Furthermore, while inspecting the width calculation in that crate I actually noticed some inefficiencies:
* Emoji presentation is queries from a separate `ucd-tri` despite the fact that `widecharwidth` already displays all `Emoji_Presentation` emojis as double width
* A perfect HashMap is used for looking up emoji variations which is likely slower than `ucd-tri`. More importantly this introduces an extra dependency.
* For characters outside the first utf-16 plane it falls back to multiple binary searches of uncompressed tables
Compared to that `unicode-width` is very lightweight as it has no extra dependencies and width calculation just compiles to a O(1) lookup in a compressed three level table (somewhat similar to `ucd-tri`).

The goal of this crate is to **combine the advantages of both**. It implements the same notion of width as `termwiz` does. However, this crate generates its own compressed lookup table just like `unicode-width` (just with different content). Emoji variations are implemented using a single `ucd-tri`. As a result this crate is very lightweight (only depends on the tiny `ucd-tri` crate) and performant. Both crates were heavily referenced while developing `grapheme-width` and are credited here as such.

To work correctly this crate calculates the width of each grapheme individually (just like `termwiz`). For convenience a function that segments the string into its grapheme and sums up their widths is provided if the `segmentation` feature is enabled.

Unicode 14 is still quite new and therefore adjusting the presentation as described above can cause compatability problems with programs that don't support unicode 14 yet. To allow downstream crates to retain compatability with these programs `grapheme-width` requires calle to specify a unicode capability level. **Ideally this compatability level should be runtime configurable as there is no standard way to negotiate a unicode version**.

# MSRV policy

The MSRV required to build `grapheme-width` is 1.65.
The MSRC increased conservatively when necessary (rarely).
It will never exceed the MSRV required by firefox to remain compatible with a wide variety of Linux distros.

Not that the MSRV policy does not apply to the `xtask` build script or `dev-dependencies`
as these are only used by dependencies and don't affect downstream crates.