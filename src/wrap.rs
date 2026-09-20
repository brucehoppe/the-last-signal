/// Word-wrap `s` to at most `width` characters per line. Paragraph breaks are
/// kept, and a word longer than a whole line is broken rather than overflowing.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = vec![];
    for paragraph in s.lines() {
        let mut line = String::new();
        let mut n = 0;
        for word in paragraph.split(' ') {
            let len = word.chars().count();
            if n > 0 && n + 1 + len <= width {
                line.push(' ');
                line.push_str(word);
                n += 1 + len;
                continue;
            }
            if n > 0 {
                out.push(std::mem::take(&mut line));
            }
            let mut rest = word;
            while rest.chars().count() > width {
                let cut = rest
                    .char_indices()
                    .nth(width)
                    .map_or(rest.len(), |(i, _)| i);
                out.push(rest[..cut].to_string());
                rest = &rest[cut..];
            }
            line = rest.to_string();
            n = rest.chars().count();
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn breaks_at_spaces_not_inside_words() {
        assert_eq!(wrap("The quick brown fox", 10), ["The quick", "brown fox"]);
        assert_eq!(wrap("a b c", 100), ["a b c"]);
    }
    #[test]
    fn long_words_are_broken_and_paragraphs_kept() {
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("one\ntwo", 10), ["one", "two"]);
        assert_eq!(wrap("", 10), [""]);
    }
    #[test]
    fn no_line_ever_exceeds_the_width() {
        let text = "Recover the evidence, transmit the last signal, and return to the surface lift. Supercalifragilisticexpialidocious!";
        for width in 1..60 {
            assert!(
                wrap(text, width).iter().all(|l| l.chars().count() <= width),
                "width {width}"
            );
        }
    }
    #[test]
    fn handles_multibyte_text() {
        assert!(wrap("naïve café ünïcode wörds", 6)
            .iter()
            .all(|l| l.chars().count() <= 6));
    }
}
