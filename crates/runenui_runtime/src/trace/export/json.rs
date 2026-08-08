use core::fmt::Write as _;

pub(super) fn string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", character as u32)
                    .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

pub(super) fn name(output: &mut String, key: &str) {
    string(output, key);
    output.push(':');
}

pub(super) fn optional_u64(output: &mut String, value: Option<u64>) {
    if let Some(value) = value {
        write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
    } else {
        output.push_str("null");
    }
}

pub(super) fn optional_usize(output: &mut String, value: Option<usize>) {
    if let Some(value) = value {
        write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
    } else {
        output.push_str("null");
    }
}

pub(super) fn optional_string(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        string(output, value);
    } else {
        output.push_str("null");
    }
}

pub(super) fn bool_value(output: &mut String, value: bool) {
    output.push_str(if value { "true" } else { "false" });
}

pub(super) fn usize_value(output: &mut String, value: usize) {
    write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
}

pub(super) fn u64_value(output: &mut String, value: u64) {
    write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
}

pub(super) fn u32_value(output: &mut String, value: u32) {
    write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
}

pub(super) fn f32_value(output: &mut String, value: f32) {
    debug_assert!(value.is_finite());
    write!(output, "{value}").unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
}
