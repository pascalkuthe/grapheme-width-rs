use std::collections::HashSet;
use std::mem::swap;
use std::ops::RangeInclusive;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use ucd_trie::{TrieSetOwned, TrieSetSlice};
use xshell::Shell;

use crate::flags::GenTables;
use crate::reformat;

const NUM_CODEPOINTS: u32 = 0x110000;
const MAX_CODEPOINT_BITS: u32 = u32::BITS - (NUM_CODEPOINTS - 1).leading_zeros();
type UncompressedTable = [u8; NUM_CODEPOINTS as usize];

fn retrieve_file(version: &str, file: &str) -> Result<String> {
    let url = format!("http://unicode.org/Public/{version}/ucd/{file}.txt");
    println!("downloading {url}...");
    ureq::get(&url)
        .call()?
        .into_string()
        .context("download failed")
}

fn parse_codepoint(s: &str) -> Result<usize> {
    usize::from_str_radix(s, 16).context("failed to parse codepoint")
}

fn parse_codepoints(s: &str) -> anyhow::Result<RangeInclusive<usize>> {
    let (start, end) = match s.split_once("..") {
        Some(range) => range,
        None => (s, s),
    };
    let start = parse_codepoint(start)?;
    let end = parse_codepoint(end)?;
    Ok(start..=end)
}

fn parse_data_line(mut line: &str) -> Option<Vec<&str>> {
    line = line.trim();
    if line.starts_with('#') || line.is_empty() {
        return None;
    }
    let line = line.split_once('#').map_or(line, |(line, _comment)| line);
    Some(line.split(';').map(str::trim).collect())
}

struct RawUnicodeData {
    /// Contents of UnicodeData.txt used to retrieve basic categories
    unicode_data: String,
    /// Contents of EastAsianWidth.txt used to retrieve east asian widths
    eaw_data: String,
    /// Contents of emoji-data.txt used to retrieve emoji presentation
    emoji_data: String,
    /// Contents of emoji-variants.txt used to retrieve emojis whose presentation
    /// and width is determined by a variant selector
    emoji_variants: String,
}

impl RawUnicodeData {
    pub fn new(version: &str) -> Result<RawUnicodeData> {
        let data = RawUnicodeData {
            unicode_data: retrieve_file(version, "UnicodeData")?,
            eaw_data: retrieve_file(version, "EastAsianWidth")?,
            emoji_data: retrieve_file(version, "emoji/emoji-data")?,
            emoji_variants: retrieve_file(version, "emoji/emoji-variation-sequences")?,
        };
        Ok(data)
    }

    fn codepoint_data(&self) -> Result<CodePointData> {
        println!("calculating codepoint widths...");
        let mut table: Box<UncompressedTable> =
            vec![u8::MAX; NUM_CODEPOINTS as usize].try_into().unwrap();
        self.fill_table_with_eaw_width(&mut table)?;
        self.fill_zero_width_categories(&mut table)?;
        self.fill_emojis(&mut table)?;
        Self::fill_hardcoded_widths(&mut table);
        let emoji_variations = self.emoji_variations()?;
        Ok(CodePointData {
            widths: table,
            emoji_variations,
        })
    }

    fn fill_hardcoded_widths(table: &mut UncompressedTable) {
        // hardcoded zero width chars: surrage pairs and private ranges count here
        let mut zerow_width_ranges = vec![
            // surrogate
            0xD800..=0xDBFF,
            0xDC00..=0xDFFF,
            // Override for Hangul Jamo medial vowels & final consonants
            // This is likely not required as we cap the grapheme width to two
            // but better save than sorry
            0x1160..=0x11FF,
        ];
        // See "noncharacters" discussion at https://www.unicode.org/faq/private_use.html
        // "Last two code points of each of the 16 supplementary planes" and also BMP (plane 0).
        zerow_width_ranges.push(0xFDD0..=0xFDEF);
        for plane in 0..=16 {
            let codepoint = 0x10000 * plane + 0xFFFE;
            zerow_width_ranges.push(codepoint..=codepoint + 1)
        }
        for zero_width in zerow_width_ranges {
            table[zero_width].fill(0)
        }
        // Override for soft hyphen
        table[0x00AD] = 1;
    }

    fn fill_table_with_eaw_width(&self, table: &mut UncompressedTable) -> Result<()> {
        for line in self.eaw_data.lines() {
            let Some(fields) = parse_data_line(line) else { continue };
            let [codepoints, width] = fields.as_slice() else { continue };
            let codepoints = parse_codepoints(codepoints)?;
            let width = if matches!(*width, "F" | "W") { 2 } else { 1 };
            table[codepoints].fill(width);
        }

        // Apply the following special cases:
        //  - The unassigned code points in the following blocks default to "W":
        //         CJK Unified Ideographs Extension A: U+3400..U+4DBF
        //         CJK Unified Ideographs:             U+4E00..U+9FFF
        //         CJK Compatibility Ideographs:       U+F900..U+FAFF
        //  - All undesignated code points in Planes 2 and 3, whether inside or
        //      outside of allocated blocks, default to "W":
        //         Plane 2:                            U+20000..U+2FFFD
        //         Plane 3:                            U+30000..U+3FFFD
        let wide_ranges = [
            0x3400..=0x4DBF,
            0x4E00..=0x9FFF,
            0xF900..=0xFAFF,
            0x20000..=0x2FFFD,
            0x30000..=0x3FFFD,
        ];
        for wide_range in wide_ranges {
            for code_point in wide_range {
                if table[code_point] == u8::MAX {
                    table[code_point] = 2
                }
            }
        }
        Ok(())
    }

    fn fill_zero_width_categories(&self, table: &mut UncompressedTable) -> Result<()> {
        for line in self.unicode_data.lines() {
            let Some(fields) = parse_data_line(line) else { continue };
            let [codepoints, _, category, ..] = fields.as_slice() else {continue;};
            let codepoints = parse_codepoints(codepoints)?;
            if matches!(
                *category,
                "Cc" | "Cf" | "Zl" | "Zp" | "Cs" | "Mn" | "Mc" | "Me"
            ) {
                table[codepoints].fill(0)
            }
        }

        Ok(())
    }

    fn fill_emojis(&self, table: &mut UncompressedTable) -> Result<()> {
        for line in self.emoji_data.lines() {
            let Some(fields) = parse_data_line(line) else { continue };
            let [codepoints, prop, ..] = fields.as_slice() else {bail!("invalid emoji data line {line}");};
            let codepoints = parse_codepoints(codepoints)?;
            // emoji presentation emojis are width 2
            if *prop == "Emoji_Presentation" {
                table[codepoints].fill(2);
            }
        }
        Ok(())
    }

    fn emoji_variations(&self) -> Result<HashSet<u32>> {
        let mut emoji_variations = HashSet::with_capacity(1024);
        for line in self.emoji_variants.lines() {
            let Some(fields) = parse_data_line(line) else { continue };
            let [codepoints, ..] = fields.as_slice() else {bail!("invalid emoji variations line {line}");};
            let codepoints: Result<Vec<_>> = codepoints.split(' ').map(parse_codepoint).collect();
            let Ok(&[emoji, 0xFE0E | 0xFE0F]) = codepoints.as_deref() else { bail!("invalid emoji variations line {line}") };
            emoji_variations.insert(emoji as u32);
        }
        Ok(emoji_variations)
    }
}

struct CodePointData {
    widths: Box<UncompressedTable>,
    emoji_variations: HashSet<u32>,
}

const TABLE_DEPTH: usize = 3;
const TABLES: [(u32, u32); TABLE_DEPTH] = [(13, MAX_CODEPOINT_BITS), (6, 13), (0, 6)];

impl CodePointData {
    fn compress_emoji_variations(&self) -> TrieSetOwned {
        println!("Compressing emoji variations...");
        TrieSetOwned::from_codepoints(self.emoji_variations.iter()).unwrap()
    }

    fn compress_widths(&self) -> [Table; TABLE_DEPTH] {
        let widths: Vec<_> = self
            .widths
            .iter()
            .copied()
            .enumerate()
            .map(|(codepoint, mut width)| {
                if width == u8::MAX {
                    width = 1
                }
                (codepoint as u32, width)
            })
            .collect();
        let mut codepoint_groups = vec![widths];
        let mut i = 0;
        TABLES.map(|(low_bit, cap_bit)| {
            println!("Compressing width table (depth {i})...");
            let table = Table::new(&codepoint_groups, low_bit, cap_bit);
            println!("found {} unique subtables", table.buckets.len());
            codepoint_groups = table
                .buckets
                .iter()
                .map(|bucket| bucket.codepoints())
                .collect();
            i += 1;
            table
        })
    }
}

const BITS_PER_CODEPOINT: u8 = 2;

#[derive(Debug)]
struct Table {
    entries: Vec<usize>,
    buckets: Vec<Bucket>,
}

impl Table {
    fn new(codepoints_groups: &[Vec<(u32, u8)>], low_bit: u32, cap_bit: u32) -> Self {
        let mut buckets = Vec::new();
        for codepoints in codepoints_groups {
            buckets.extend_from_slice(&Bucket::for_bits(codepoints, low_bit, cap_bit));
        }
        let mut merged_buckets: Vec<Bucket> = Vec::new();
        let mut bucket_indecies = Vec::new();
        'outer: for bucket in buckets {
            for (i, other_bucket) in merged_buckets.iter_mut().enumerate() {
                if other_bucket.try_merge(&bucket) {
                    bucket_indecies.push(i);
                    continue 'outer;
                }
            }
            bucket_indecies.push(merged_buckets.len());
            merged_buckets.push(bucket);
        }
        Table {
            entries: bucket_indecies,
            buckets: merged_buckets,
        }
    }

    fn into_flat_bytes(self) -> Vec<u8> {
        assert_eq!(
            self.entries.len() % (u8::BITS as u8 / BITS_PER_CODEPOINT) as usize,
            0
        );
        assert_eq!(BITS_PER_CODEPOINT, 2);
        self.entries
            .chunks_exact(4)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .map(|(i, &bucket)| {
                        let width = self.buckets[bucket].width().unwrap();
                        assert!((u8::BITS - width.leading_zeros()) as u8 <= BITS_PER_CODEPOINT);
                        width << (i as u8 * BITS_PER_CODEPOINT)
                    })
                    .sum()
            })
            .collect()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.entries
            .iter()
            .map(|&i| u8::try_from(i).unwrap())
            .collect()
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    codepoints: Vec<(u32, u8)>,
    widths: Vec<u8>,
}

impl Bucket {
    fn for_bits(codepoints: &[(u32, u8)], low_bit: u32, cap_bit: u32) -> Vec<Bucket> {
        let num_bits = cap_bit - low_bit;
        let mask = (1 << num_bits) - 1;
        let mut buckets = vec![
            Bucket {
                codepoints: Vec::new(),
                widths: Vec::new()
            };
            2 << (num_bits - 1)
        ];
        for &(codepoint, width) in codepoints {
            let bucket = &mut buckets[((codepoint >> low_bit) & mask) as usize];
            bucket.codepoints.push((codepoint, width));
            bucket.widths.push(width);
        }

        buckets
    }

    fn try_merge(&mut self, other: &Bucket) -> bool {
        let (mut less, mut more) = (&*self, other);
        if less.widths.len() > more.widths.len() {
            swap(&mut less, &mut more);
        }
        if less.widths().eq(more.widths().take(less.codepoints.len())) {
            self.widths = more.widths.clone();
            self.codepoints.extend(other.codepoints.iter().copied());
            true
        } else {
            false
        }
    }
    fn codepoints(&self) -> Vec<(u32, u8)> {
        let mut codepoints = self.codepoints.clone();
        codepoints.sort_unstable_by_key(|&(codepoint, _)| codepoint);
        codepoints
    }

    fn widths(&self) -> impl Iterator<Item = u8> + '_ {
        self.widths.iter().copied()
    }

    fn width(&self) -> Option<u8> {
        let width_0 = *self.widths.first()?;
        self.widths
            .iter()
            .all(|&width| width == width_0)
            .then_some(width_0)
    }
}

fn emit_width_table(tables: [Table; TABLE_DEPTH], version: &str) -> Result<TokenStream> {
    let mut res = TokenStream::new();
    let version_components: Result<Vec<_>, _> =
        version.trim().split('.').map(u8::from_str).collect();
    let Ok([major, minor, patch]) = version_components.as_deref() else { bail!("Invalid version {version}") };
    quote! {
        /// Version of the UCD used to generate the width lookup tables
        pub const UNICODE_VERSION: (u8, u8, u8) = (#major, #minor, #patch);
    }
    .to_tokens(&mut res);
    for (i, table) in tables.into_iter().enumerate() {
        let table = if i == TABLE_DEPTH - 1 {
            table.into_flat_bytes()
        } else {
            table.into_bytes()
        };
        let table_name = format_ident!("TABLE_{i}");
        let table_len = table.len();
        quote! {
            pub(crate) static #table_name: [u8; #table_len]  = [#(#table),*];
        }
        .to_tokens(&mut res)
    }

    Ok(res)
}

fn emit_emoji_variations(set: TrieSetOwned) -> TokenStream {
    let TrieSetSlice {
        tree1_level1,
        tree2_level1,
        tree2_level2,
        tree3_level1,
        tree3_level2,
        tree3_level3,
    } = set.as_slice();
    quote! {
        pub(crate) const EMOJI_VARIATIONS: &'static ::ucd_trie::TrieSet = &::ucd_trie::TrieSet {
            tree1_level1: &[#(#tree1_level1),*],
            tree2_level1: &[#(#tree2_level1),*],
            tree2_level2: &[#(#tree2_level2),*],
            tree3_level1: &[#(#tree3_level1),*],
            tree3_level2: &[#(#tree3_level2),*],
            tree3_level3: &[#(#tree3_level3),*],
        };
    }
    .to_token_stream()
}

impl GenTables {
    pub fn run(self, sh: &Shell) -> Result<()> {
        let version = self.unicode_version;
        println!("generating tables for Unicode {version}");
        let raw_data = RawUnicodeData::new(&version)?;
        let code_point_data = raw_data.codepoint_data()?;
        let width_tables = code_point_data.compress_widths();
        let emoji_variations = code_point_data.compress_emoji_variations();
        println!("generating table.rs...");
        let table = emit_width_table(width_tables, &version)?;
        let table = reformat(sh, table.to_string());
        let table = format!("//! Generated by `cargo xtask gen-tables`, do not edit by hand.\n//! This file contains a three level LUT for determining the display width of a unicode grapheme.\n//! It was generated from UCD {version}\n\n{table}");
        sh.write_file("src/table.rs", table)?;
        println!("generating emoji_variations.rs...");
        let emoji_variations = emit_emoji_variations(emoji_variations);
        let emoji_variations = reformat(sh, emoji_variations.to_string());
        let emoji_variations = format!("//! Generated by `cargo xtask gen-tables`, do not edit by hand.\n//! This file contains a UCD tri-set for determining whether an emojis presentation can be controlled with VS15/VS16.\n//! It was generated from UCD {version}\n\n{emoji_variations}");
        sh.write_file("src/emoji_variations.rs", emoji_variations)?;
        Ok(())
    }
}
