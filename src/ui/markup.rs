//! Минимальная разметка текста курса: `**жирный**`, `*курсив*`, `` `код` `` и `[ссылка](https://…)`.

use eframe::egui::{
    self,
    text::{LayoutJob, TextFormat},
    Color32, RichText, Stroke,
};

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
            // курсив: открывающая `*` не внутри слова, закрывающая не перед буквой и не рядом с другой `*`;
            // так указатели в C (`char *p`, `(*p)`) остаются текстом
            let opens = input[..i]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            body.find('*')
                .filter(|&n| {
                    let after = body[n + 1..].chars().next();
                    opens
                        && n > 0
                        && !body.starts_with(char::is_whitespace)
                        && !body[..n].ends_with(char::is_whitespace)
                        && after.is_none_or(|c| !c.is_alphanumeric() && c != '*')
                })
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

/// Текст без знаков разметки (для заголовков и списков, где формат не нужен).
pub fn strip_marks(input: &str) -> String {
    parse(input)
        .into_iter()
        .map(|span| match span {
            Span::Text(t) | Span::Bold(t) | Span::Italic(t) | Span::Code(t) => t,
            Span::Link { label, .. } => label,
        })
        .collect()
}

/// Обычный формат текста виджета: цвет подставит сам виджет.
pub fn format_for(ui: &egui::Ui) -> TextFormat {
    TextFormat {
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        color: Color32::PLACEHOLDER,
        ..TextFormat::default()
    }
}

/// Та же разметка для виджетов, которые принимают только текст (флажки, метки). Ссылки в них не
/// кликабельны: рисуются подчёркнутыми.
pub fn job(ui: &egui::Ui, input: &str, base: TextFormat) -> LayoutJob {
    let visuals = &ui.style().visuals;
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = LayoutJob::default();
    for span in parse(input) {
        let (text, format) = match span {
            Span::Text(t) => (t, base.clone()),
            Span::Bold(t) => (
                t,
                TextFormat {
                    color: visuals.strong_text_color(),
                    ..base.clone()
                },
            ),
            Span::Italic(t) => (
                t,
                TextFormat {
                    italics: true,
                    ..base.clone()
                },
            ),
            Span::Code(t) => (
                t,
                TextFormat {
                    font_id: mono.clone(),
                    background: visuals.code_bg_color,
                    ..base.clone()
                },
            ),
            Span::Link { label, .. } => (
                label,
                TextFormat {
                    color: visuals.hyperlink_color,
                    underline: Stroke::new(1.0, visuals.hyperlink_color),
                    ..base.clone()
                },
            ),
        };
        job.append(text, 0.0, format);
    }
    job
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
    fn c_pointers_are_not_emphasis() {
        for s in [
            "char *p = data + 5; while (*p) p++;",
            "читаю *p** и **p[i]** бегло",
            "int **argv, *p;",
        ] {
            let spans = parse(s);
            assert!(
                spans.iter().all(|sp| !matches!(sp, Span::Italic(_))),
                "{s}: {spans:?}"
            );
        }
        assert_eq!(
            parse("(*важно*) и *ещё*."),
            vec![
                Span::Text("("),
                Span::Italic("важно"),
                Span::Text(") и "),
                Span::Italic("ещё"),
                Span::Text("."),
            ]
        );
    }

    #[test]
    fn strip_marks_drops_the_marks_but_keeps_the_words() {
        assert_eq!(
            strip_marks("**жирный**, *курсив*, `код` и [ссылка](https://x.io)"),
            "жирный, курсив, код и ссылка"
        );
        assert_eq!(strip_marks("2*3*4"), "2*3*4");
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

    /// Обратная сборка: разметка → исходный текст.
    fn to_source(spans: &[Span]) -> String {
        spans
            .iter()
            .map(|s| match s {
                Span::Text(t) => (*t).to_string(),
                Span::Bold(t) => format!("**{t}**"),
                Span::Italic(t) => format!("*{t}*"),
                Span::Code(t) => format!("`{t}`"),
                Span::Link { label, url } => format!("[{label}]({url})"),
            })
            .collect()
    }

    #[test]
    fn parsing_is_lossless_for_all_course_text() {
        let c = crate::curriculum::Curriculum::load();
        let mut with_markup = 0;
        for w in &c.weeks {
            let lab_steps = w.lab.iter().flat_map(|l| l.steps.iter());
            let lines = w
                .lectures
                .iter()
                .chain(w.case.iter())
                .chain(lab_steps)
                .chain(w.psets.iter())
                .chain(w.checkpoint.iter());
            for line in lines {
                let spans = parse(line);
                assert_eq!(
                    to_source(&spans),
                    *line,
                    "{}: разбор потерял или исказил текст",
                    w.id
                );
                // курсив в курсе — это слова; всё с кодом внутри — случайно съеденные звёздочки
                for span in &spans {
                    if let Span::Italic(t) = span {
                        assert!(
                            !t.contains(['=', ';', '(', ')', '[', ']', '{', '}', '<', '>', '+']),
                            "{}: курсив съел код: *{t}* (оберните код в обратные кавычки)",
                            w.id
                        );
                    }
                }
                with_markup += spans.iter().filter(|s| !matches!(s, Span::Text(_))).count();
            }
        }
        assert!(
            with_markup >= 20,
            "в курсе должна быть разметка (ссылки, жирный, курсив): {with_markup}"
        );
    }
}
