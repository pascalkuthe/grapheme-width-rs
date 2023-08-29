use crate::str_width;
use crate::{UnicodeCompat::Unicode14, UnicodeCompat::Unicode9};

#[test]
fn issue_1161() {
    assert_eq!(str_width("\u{3000}", Unicode14), 2);
}

#[test]
fn issue_997() {
    let victory_hand = "\u{270c}";
    let victory_hand_text_presentation = "\u{270c}\u{fe0e}";

    assert_eq!(str_width(victory_hand_text_presentation, Unicode14), 1);
    assert_eq!(str_width(victory_hand, Unicode14), 1);

    let copyright_emoji_presentation = "\u{00A9}\u{FE0F}";
    assert_eq!(str_width(copyright_emoji_presentation, Unicode14), 2);
    assert_eq!(str_width(copyright_emoji_presentation, Unicode9), 1);

    let copyright_text_presentation = "\u{00A9}";
    assert_eq!(str_width(copyright_text_presentation, Unicode14), 1);

    let raised_fist = "\u{270a}";
    // Not valid to have explicit Text presentation for raised fist
    let raised_fist_text = "\u{270a}\u{fe0e}";
    assert_eq!(str_width(raised_fist, Unicode14), 2);
    assert_eq!(str_width(raised_fist_text, Unicode14), 2);
}

#[test]
fn issue_1573() {
    let sequence = "\u{1112}\u{1161}\u{11ab}";
    assert_eq!(str_width(sequence, Unicode14), 2);
    assert_eq!(str_width(sequence, Unicode9), 2);

    let sequence2 = std::str::from_utf8(b"\xe1\x84\x92\xe1\x85\xa1\xe1\x86\xab").unwrap();
    assert_eq!(str_width(sequence2, Unicode14), 2);
    assert_eq!(str_width(sequence2, Unicode9), 2);
}

#[test]
fn issue_5502() {
    // some emulators have historally treated this as double width even tough it isn't
    // ensure that we treat this as single width and that wezterm/termwiz does too
    assert_eq!(str_width("🗙", Unicode9), 1);
    assert_eq!(str_width("🗙", Unicode14), 1);
    assert_eq!(termwiz::cell::grapheme_column_width("🗙", None), 1);
}

#[test]
fn emoji_representation() {
    // its annoying but we don't grapheme segment so each emoji must be calcultade indivudlaly
    assert_eq!(str_width("👩‍❤️‍👨", Unicode9), 5);
    assert_eq!(str_width("👩‍❤️‍👨", Unicode14), 6);
    assert_eq!(str_width("✔️", Unicode9), 1);
    assert_eq!(str_width("✔️", Unicode14), 2);
}
