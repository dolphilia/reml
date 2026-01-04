use reml_runtime::text::{width_map_with_stats, Str, WidthMode};

const EAST_ASIAN_WIDTH_DATA: &str =
    include_str!("../../../tests/data/unicode/UCD/EastAsianWidth-15.1.0.txt");

#[test]
fn east_asian_width_classes_are_respected() {
    for line in EAST_ASIAN_WIDTH_DATA.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (range, class) = match trimmed.split_once(';') {
            Some(parts) => (parts.0.trim(), parts.1.trim()),
            None => continue,
        };
        let Some(expected) = expected_width_for_class(class) else {
            continue;
        };
        let (start, end) = parse_range(range);
        for codepoint in start..=end {
            if (0xD800..=0xDFFF).contains(&codepoint) {
                continue;
            }
            let Some(ch) = char::from_u32(codepoint) else {
                continue;
            };
            let buffer = ch.to_string();
            let str_ref = Str::from(buffer.as_str());
            let (_, stats) = width_map_with_stats(&str_ref, WidthMode::Wide);
            assert_eq!(
                stats.corrected_width, expected,
                "unexpected width for U+{:04X} class {class}",
                codepoint
            );
        }
    }
}

fn parse_range(range: &str) -> (u32, u32) {
    if let Some((start, end)) = range.split_once("..") {
        (
            u32::from_str_radix(start, 16).expect("invalid start"),
            u32::from_str_radix(end, 16).expect("invalid end"),
        )
    } else {
        let value = u32::from_str_radix(range, 16).expect("invalid value");
        (value, value)
    }
}

fn expected_width_for_class(class: &str) -> Option<usize> {
    match class {
        "W" | "F" | "A" => Some(2),
        _ => None,
    }
}
