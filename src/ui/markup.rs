//! Минимальная разметка текста курса: `**жирный**`, `*курсив*`, `` `код` `` и `[ссылка](https://…)`.

use eframe::egui::{self, RichText};

#[derive(Debug, PartialEq, Eq)]
pub enum Span<'a> {
    Text(&'a str),
    Bold(&'a str),
    Italic(&'a str),
    Code(&'a str),
    Link { label: &'a str, url: &'a str },
}

/// Разбор одной строки. Незакрытая или пустая разметка остаётся обычным текстом.
pub fn parse(input: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let (mut text_start, mut i) = (0, 0);
    while i < input.len() {
        let rest = &input[i..];
        let found = if let Some(body) = rest.strip_prefix("**") {
            body.find("**")
                .filter(|&n| n > 0)
                .map(|n| (Span::Bold(&body[..n]), 2 + n + 2))
        } else if let Some(body) = rest.strip_prefix('*') {
            // курсив: открывающая звёздочка не внутри слова, снаружи от пробелов
            let opens = input[..i]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            body.find('*')
                .filter(|&n| opens && n > 0 && !body.starts_with(' ') && !body[..n].ends_with(' '))
                .map(|n| (Span::Italic(&body[..n]), 1 + n + 1))
        } else if let Some(body) = rest.strip_prefix('`') {
            body.find('`')
                .filter(|&n| n > 0)
                .map(|n| (Span::Code(&body[..n]), 1 + n + 1))
        } else if let Some(body) = rest.strip_prefix('[') {
            body.find("](").and_then(|close| {
                let label = &body[..close];
                let after = &body[close + 2..];
                let end = after.find(')')?;
                let url = &after[..end];
                let valid = !label.is_empty()
                    && !label.contains('[')
                    && (url.starts_with("https://") || url.starts_with("http://"));
                valid.then_some((Span::Link { label, url }, 1 + close + 2 + end + 1))
            })
        } else {
            None
        };
        match found {
            Some((span, len)) => {
                if text_start < i {
                    spans.push(Span::Text(&input[text_start..i]));
                }
                spans.push(span);
                i += len;
                text_start = i;
            }
            None => i += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    if text_start < input.len() {
        spans.push(Span::Text(&input[text_start..]));
    }
    spans
}

/// Строка с разметкой; ссылки открываются в браузере.
pub fn show(ui: &mut egui::Ui, input: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in parse(input) {
            match span {
                Span::Text(t) => ui.label(t),
                Span::Bold(t) => ui.label(RichText::new(t).strong()),
                Span::Italic(t) => ui.label(RichText::new(t).italics()),
                Span::Code(t) => ui.label(RichText::new(t).monospace()),
                Span::Link { label, url } => ui.hyperlink_to(label, url),
            };
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(spans: &[Span]) -> String {
        spans
            .iter()
            .map(|s| if let Span::Text(t) = s { *t } else { "" })
            .collect()
    }

    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(parse("просто текст"), vec![Span::Text("просто текст")]);
        assert!(parse("").is_empty());
    }

    #[test]
    fn bold_code_and_links_are_recognised() {
        assert_eq!(
            parse("**CS50x** (см. [cs50](https://cs50.harvard.edu/x/)) и `malloc`"),
            vec![
                Span::Bold("CS50x"),
                Span::Text(" (см. "),
                Span::Link {
                    label: "cs50",
                    url: "https://cs50.harvard.edu/x/"
                },
                Span::Text(") и "),
                Span::Code("malloc"),
            ]
        );
    }

    #[test]
    fn italics_need_word_boundaries() {
        assert_eq!(
            parse("*The Ghidra Book* (гл. 1)"),
            vec![Span::Italic("The Ghidra Book"), Span::Text(" (гл. 1)")]
        );
        // умножение и указатели остаются текстом
        for s in ["2*3*4", "a * b * c", "int *p, *q;"] {
            assert!(
                parse(s).iter().all(|sp| matches!(sp, Span::Text(_))),
                "{s}: {:?}",
                parse(s)
            );
        }
    }

    #[test]
    fn broken_markup_stays_text() {
        for s in [
            "**без конца",
            "`без конца",
            "[без ссылки]",
            "[a](ftp://x)",
            "[](https://x)",
            "**** пусто",
            "``",
            "[a](https://x",
        ] {
            let spans = parse(s);
            assert!(
                spans.iter().all(|sp| matches!(sp, Span::Text(_))),
                "{s}: {spans:?}"
            );
            assert_eq!(plain(&spans), s);
        }
    }

    #[test]
    fn multibyte_text_does_not_panic() {
        assert_eq!(parse("Ъ**ж**ё[я](https://я.рф)ю").len(), 5);
    }

    #[test]
    fn course_strings_never_lose_visible_text() {
        let c = crate::curriculum::Curriculum::load();
        for w in &c.weeks {
            for line in w.lectures.iter().chain(w.case.iter()) {
                let visible: usize = parse(line)
                    .iter()
                    .map(|s| match s {
                        Span::Text(t) | Span::Bold(t) | Span::Italic(t) | Span::Code(t) => {
                            t.chars().count()
                        }
                        Span::Link { label, .. } => label.chars().count(),
                    })
                    .sum();
                assert!(
                    visible > 0 && visible <= line.chars().count(),
                    "{}: {line}",
                    w.id
                );
            }
        }
    }
}
