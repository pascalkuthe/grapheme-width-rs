use crate::{
    grapheme_width, grapheme_width_non_zero, UnicodeCompat::Unicode14, UnicodeCompat::Unicode9,
};

#[test]
fn test_width() {
    let foot = "\u{1f9b6}";
    eprintln!("foot chars");
    for c in foot.chars() {
        eprintln!("char: {:?}", c);
    }
    assert_eq!(
        grapheme_width(foot, Unicode14),
        2,
        "width of {} should be 2",
        foot
    );

    let women_holding_hands_dark_skin_tone_medium_light_skin_tone =
        "\u{1F469}\u{1F3FF}\u{200D}\u{1F91D}\u{200D}\u{1F469}\u{1F3FC}";
    assert_eq!(
        grapheme_width(
            women_holding_hands_dark_skin_tone_medium_light_skin_tone,
            Unicode14
        ),
        2
    );

    let deaf_man = "\u{1F9CF}\u{200D}\u{2642}\u{FE0F}";
    eprintln!("deaf_man chars");
    for c in deaf_man.chars() {
        eprintln!("char: {:?}", c);
    }
    assert_eq!(grapheme_width(deaf_man, Unicode14), 2);

    let man_dancing = "\u{1F57A}";
    assert_eq!(grapheme_width(man_dancing, Unicode9), 2);

    let raised_fist = "\u{270a}";
    assert_eq!(grapheme_width(raised_fist, Unicode9), 2);

    // This is a codepoint in the private use area
    let font_awesome_star = "\u{f005}";
    eprintln!("font_awesome_star {}", font_awesome_star.escape_debug());
    assert_eq!(grapheme_width(font_awesome_star, Unicode14), 1);

    let england_flag = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}";
    assert_eq!(grapheme_width(england_flag, Unicode14), 2);
}

#[test]
fn issue_1161() {
    assert_eq!(grapheme_width("\u{3000}", Unicode14), 2);
}

#[test]
fn issue_997() {
    let victory_hand = "\u{270c}";
    let victory_hand_text_presentation = "\u{270c}\u{fe0e}";

    assert_eq!(grapheme_width(victory_hand_text_presentation, Unicode14), 1);
    assert_eq!(grapheme_width(victory_hand, Unicode14), 1);

    let copyright_emoji_presentation = "\u{00A9}\u{FE0F}";
    assert_eq!(grapheme_width(copyright_emoji_presentation, Unicode14), 2);
    assert_eq!(grapheme_width(copyright_emoji_presentation, Unicode9), 1);

    let copyright_text_presentation = "\u{00A9}";
    assert_eq!(grapheme_width(copyright_text_presentation, Unicode14), 1);

    let raised_fist = "\u{270a}";
    // Not valid to have explicit Text presentation for raised fist
    let raised_fist_text = "\u{270a}\u{fe0e}";
    assert_eq!(grapheme_width(raised_fist, Unicode14), 2);
    assert_eq!(grapheme_width(raised_fist_text, Unicode14), 2);
}

#[test]
fn issue_1573() {
    let sequence = "\u{1112}\u{1161}\u{11ab}";
    assert_eq!(grapheme_width(sequence, Unicode14), 2);
    assert_eq!(grapheme_width(sequence, Unicode9), 2);

    let sequence2 = std::str::from_utf8(b"\xe1\x84\x92\xe1\x85\xa1\xe1\x86\xab").unwrap();
    assert_eq!(grapheme_width(sequence2, Unicode14), 2);
    assert_eq!(grapheme_width(sequence2, Unicode9), 2);
}

#[test]
fn issue_5502() {
    // some emulators have historally treated this as double width even tough it isn't
    // ensure that we treat this as single width and that wezterm/termwiz does too
    assert_eq!(grapheme_width("🗙", Unicode9), 1);
    assert_eq!(grapheme_width("🗙", Unicode14), 1);
    assert_eq!(termwiz::cell::grapheme_column_width("🗙", None), 1);
}

#[test]
fn single_byte_fast_path() {
    for c in 0..=u8::MAX {
        if let Ok(str) = std::str::from_utf8(&[c]) {
            assert_eq!(
                grapheme_width(str, Unicode14).max(1),
                grapheme_width_non_zero(str, Unicode14)
            )
        }
    }
}
