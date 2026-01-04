use reml_runtime::text::{
    self, to_lower, to_upper, width_map_with_stats, LocaleId, Str, String as TextString, WidthMode,
};

#[test]
fn turkish_locale_converts_identifiers() {
    let locale = LocaleId::parse("tr-TR").expect("locale");
    let input = TextString::from_str("identifier iıIİ");
    let upper = to_upper(input, &locale).expect("upper");
    assert_eq!(upper.as_str(), "İDENTİFİER İIIİ");

    let lowered = to_lower(TextString::from_str("IDENTIFIER Iİ"), &locale).expect("lower");
    assert_eq!(lowered.as_str(), "ıdentıfıer ıi");
}

#[test]
fn unsupported_locale_is_rejected() {
    let locale = LocaleId::parse("az-Latn").expect("locale");
    let err = to_upper(TextString::from_str("test"), &locale).unwrap_err();
    assert_eq!(err.kind(), text::UnicodeErrorKind::UnsupportedLocale);
    assert!(
        err.message().contains("planned") || err.message().contains("partial"),
        "エラーメッセージにサポート状況が含まれていません: {}",
        err.message()
    );
}

#[test]
fn width_map_updates_stats_and_transforms_text() {
    let src = Str::from("ﾊﾝｶｸ ｶﾀｶﾅ");
    let (wide, stats) = width_map_with_stats(&src, WidthMode::Wide);
    assert_eq!(wide.as_str(), "ハンカク　カタカナ");
    assert!(stats.corrected_width >= stats.original_width);
    assert!(stats.corrections_applied >= 1);

    let (narrow, _) = width_map_with_stats(&Str::from(wide.as_str()), WidthMode::Narrow);
    assert_eq!(narrow.as_str(), "ﾊﾝｶｸ ｶﾀｶﾅ");
}
