pub(crate) fn one_line(message: &str) -> String {
    message.replace(['\r', '\n'], " ")
}

pub(crate) fn normalize_lines(message: &str) -> String {
    message.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_line_endings() {
        assert_eq!(
            normalize_lines("first\r\nsecond\rthird"),
            "first\nsecond\nthird"
        );
    }
}
