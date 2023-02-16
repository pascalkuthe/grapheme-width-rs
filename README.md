# grapheme-width

This crate provides a function to compute the number of columns occupied by a single [unicode grapheme](https://unicode.org/reports/tr29/). It's distinguishing factors are that it always returns the correct **display width**,
**backwards compatability** with older unicode versions and being **lightweight**.

There are two other options that provide similar functionality already in the rust ecosystem:
* The [unicode width](https://github.com/unicode-rs/unicode-width) crate
* The [`grapheme_column_width`](https://docs.rs/termwiz/0.20.0/termwiz/cell/fn.grapheme_column_width.html) function of the [`termwiz`](https://crates.io/crates/termwiz) crate.

Both options have drawbacks

The [unicode-width](https://github.com/unicode-rs/unicode-width) crate strictly returns the width as definied in [Unicode Standard Annex #11](https://www.unicode.org/reports/tr11/). This is a valid usecase on its own. However many applications are interested in the actual display width instead. For example unicode with return width of 5 for the following emoji `🤦🏼‍♂️` wheras this emoji only has a displaywidth of two. It also doesn't account for emoji presentation/width changes caused by emoji variation selectors VS15 and VS16 introduced in unicode 14. For example the following emojis have text presentation by default but because they are followed by VS16 they are switched to emoji presentation and double width: ✔️ 🖋️.

To handle the cases described above the [`termwiz`](https://crates.io/crates/termwiz) crate (part of the [`wezterm`](https://github.com/wez/wezterm) emulator) switch to a custom grapheme width calculation based on [widecharwidth](https://github.com/ridiculousfish/widecharwidth/). To account for unicode 14 presentation changes handeling for emoji variations were also added. However the termwiz crate is a very heavy dependency. Not only does it contain a LOT functionality itself it also also a large number of (transitive) dependencies. Furthermore while inspecting the width calculation in that crate I actually noticed some inefficencies:
* Emoji presentation is queries from a seperate `ucd-tri` despite the fact that `widecharwidth` already displays all `Emoji_Presentation` emojis as double width
* A perfect hashmap is used for looking up emoji variations which is likely slower than `ucd-tri`. More importantly this introduces an extra dependency.
* For characters outside of the first utf-16 plane it falls back to multiple binary searches of uncompressed tables
Compared to that unicode-width is very ligthweight as it has no extra dependencies and width calculation just compiles to a O(1) lookup in a compressed three level table (somewhat similar to `ucd-tri`).

The goal of this crate is to **combine the advantages of both**. It implements the same notion of width as `termwiz` does. However this crate generates it's own compressed lookup table just like `unicode-width` (just with different content). Emoji variations are implemented using a single `ucd-tri`. As a result this crate is very leightweight (only depends on the tiny `ucd-tri` crate) and performant. Both crates were heavily referenced while developing `grapheme-width` and are credited here as such.

To work correctly this crate calculates the width of each grapheme individually (just like `termwiz`). For convenience a function that segements the string into its graphemes and sums up their widths is provided if the `segmentation` feature is enabled.

Unicode 14 is still quite new and therefore adjusting the presentation as described above can cause compatability problems with programs that don't support unicode 14 yet. To allow downstream crates to retain compatability with these programs `grapheme-width` requires calle to specify a unicode capability level. **Ideally this compatability level should be runtime configurable as there is no standard way to negotiate a unicode version**.

# MSRV policy

The MSRV required to build `grapheme-width` is 1.63.
The MSRC increased conservatively when necessary (rarely).
It will never exceed the MSRV required by firefox to remain compatbile with a wide variety of linux distros.

Not that the MSRV policy does not apply to the `xtask` build script or `dev-dependencies`
as these are only used by dependencies and don't affect downstream crates.