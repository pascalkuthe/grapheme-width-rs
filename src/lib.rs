#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use crate::emoji_variations::EMOJI_VARIATIONS;

#[allow(warnings)]
mod emoji_variations;
#[allow(warnings)]
mod table;
#[cfg(test)]
mod test;

pub use table::UNICODE_VERSION;

/// Controls backwards compatability with older Unicode version.
/// The core width lookup tables are always generated from the newest
/// unicode version, see [`crate::UNICODE_VERSION`]. For the most part
/// this should not be a problem when targeting older versions as
/// unicode width changes are backwards compatible.
///
/// However the width of some emojis changed in some unicode versions.
/// To avoid visual artificats when rendering to a terminal
/// make sure that the right version is selected here (ideally offer a config option).
/// Usally defaulting to `Unicode9` is a good idea when targeting the terminal
/// and hence returned by `UncodeCompat::default(). Only a few emulators use unicode 14
/// emoji width (see documentation of `UnicodeCompact::Unicode14`).
///
/// Note that backwards compatability for legacy unicode versions before Unicode 9
/// is not provided
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum UnicodeCompat {
    /// Compatiable with Unicode Versions 9 to 14.
    ///
    /// As of early 2023 this setting is correct for the following emulators (non exhaustive):
    /// `wezterm` (configurable), `alacritty`, `gnome console`, `kde konsole` and `iterm`.
    ///
    /// Variant selectors can not change the presentation/width of emojis
    #[default]
    Unicode9,
    /// Compatible with Unicode Version 14+
    ///
    /// As of early 2023 this setting is correct for the following emulators (non exhaustive):
    /// `kitty`, `windows cmd`, `windows powershell` and `windows terminal`.
    ///
    /// With this compatability level emoji variant selectors
    /// can change the presentation of some emojis
    /// between text presentation (width 1) and emoji presentation (width 2)
    Unicode14,
}

/// Computes the width of a string
#[inline]
pub fn str_width(s: &str, unicode_compact: UnicodeCompat) -> usize {
    let mut chars = s.chars();
    match unicode_compact {
        UnicodeCompat::Unicode9 => chars.map(char_width_unicode9).sum(),
        UnicodeCompat::Unicode14 => {
            let mut res = 0;
            while let Some(c) = chars.next() {
                println!("{c:?}");
                if c.is_ascii() {
                    res += (!(c as u8).is_ascii_control()) as usize;
                    continue;
                }
                // For unicode 14 respect emoji-variations.txt
                // If there is no explicit variant select then the default width algorithm always
                // returns the width for the default presentation so no need to specical case
                if EMOJI_VARIATIONS.contains_char(c) {
                    match chars.as_str().as_bytes() {
                        // text variant select U-FE0E as bytes
                        [0xef, 0xb8, 0x8e, ..] => {
                            chars = chars.as_str()[3..].chars();
                            res += 1;
                            continue;
                        }
                        // emoji variant select U-FE0F as bytes
                        [0xef, 0xb8, 0x8f, ..] => {
                            chars = chars.as_str()[3..].chars();
                            res += 2;
                            continue;
                        }
                        _ => (),
                    }
                }

                let width = lookup_width(c) as usize;
                res += width;
            }
            res
        }
    }
}

#[inline]
fn lookup_width(c: char) -> u8 {
    use table::*;
    let cp = c as usize;

    let t1_offset = TABLE_0[cp >> 13 & 0xFF];

    // Each sub-table in TABLES_1 is 7 bits, and each stored entry is a byte,
    // so each sub-table is 128 bytes in size.
    // (Sub-tables are selected using the computed offset from the previous table.)
    let t2_offset = TABLE_1[128 * usize::from(t1_offset) + (cp >> 6 & 0x7F)];

    // Each sub-table in TABLES_2 is 6 bits, but each stored entry is 2 bits.
    // This is accomplished by packing four stored entries into one byte.
    // So each sub-table is 2**(6-2) == 16 bytes in size.
    // Since this is the last table, each entry represents an encoded width.
    let packed_widths = TABLE_2[16 * usize::from(t2_offset) + (cp >> 2 & 0xF)];

    // Extract the packed width
    packed_widths >> (2 * (cp & 0b11)) & 0b11
}

/// Calculates the width of a single character. This never takes text represeentation
/// into account and therefore implies `UnicodeCompat::Unicode9`. For non-emoji
/// characters this is equivalent to [`char_width_unicode14`].
#[inline]
pub fn char_width_unicode9(c: char) -> usize {
    if c.is_ascii() {
        return (!(c as u8).is_ascii_control()) as usize;
    }
    lookup_width(c) as usize
}

/// Calculates the width of a single character that is followed by a text
/// representation character. This never takes text represeentation into account
/// and therefore implies `UnicodeCompat::Unicode14`. For non-emoji
/// characters this is equivalent to [`char_width_unicode9`].
#[inline]
pub fn char_width_unicode14(c: char, rem: &str) -> usize {
    if c.is_ascii() {
        return (!(c as u8).is_ascii_control()) as usize;
    }
    // For unicode 14 respect emoji-variations.txt
    // If there is no explicit variant select then the default width algorithm always
    // returns the width for the default presentation so no need to specical case
    if EMOJI_VARIATIONS.contains_char(c) {
        match rem.as_bytes() {
            // text variant select U-FE0E as bytes
            [0xef, 0xb8, 0x8e, ..] => return 1,
            // emoji variant select U-FE0F as bytes
            [0xef, 0xb8, 0x8f, ..] => return 2,
            _ => (),
        }
    }
    lookup_width(c) as usize
}
