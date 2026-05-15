use crate::models::{App, InputMode};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub fn helix_previous_word(app: &mut App) {
    let old_index = app.state.user_input.character_index;
    let text_before_cursor = &app.state.user_input.input[..old_index];
    let trimmed = text_before_cursor.trim_end();

    let new_index = if trimmed.is_empty() {
        0
    } else {
        match trimmed.rfind(' ') {
            Some(space_index) => space_index + 1,
            None => 0,
        }
    };

    app.state.user_input.character_index = new_index;

    if new_index != old_index {
        app.state.user_input.highlight_range = Some((new_index, old_index));
    } else {
        app.state.user_input.highlight_range = None;
    }
}

pub fn helix_next_word(app: &mut App) {
    let old_index = app.state.user_input.character_index;
    let char_count = app.state.user_input.input.chars().count();

    let text_after_cursor: String = app.state.user_input.input.chars().skip(old_index).collect();

    let first_space = text_after_cursor
        .char_indices()
        .find(|(_, c)| c.is_whitespace());

    let new_index = if let Some((space_index, _)) = first_space {
        let remaining = &text_after_cursor[space_index..];
        let next_word_start = remaining.char_indices().find(|(_, c)| !c.is_whitespace());

        if let Some((start_index, _)) = next_word_start {
            old_index + space_index + start_index
        } else {
            char_count
        }
    } else {
        char_count
    };

    app.state.user_input.character_index = new_index;

    if new_index != old_index {
        app.state.user_input.highlight_range = Some((old_index, new_index));
    } else {
        app.state.user_input.highlight_range = None;
    }
}

pub fn build_input_spans(
    text: &str,
    cursor_pos: usize,
    highlight_range: Option<(usize, usize)>,
    input_mode: &InputMode,
) -> Line<'static> {
    let chars: Vec<char> = text.chars().collect();

    match input_mode {
        InputMode::Editing => {
            let mut spans = Vec::new();
            let before: String = chars[..cursor_pos.min(chars.len())].iter().collect();
            let cursor_char = if cursor_pos < chars.len() {
                chars[cursor_pos].to_string()
            } else {
                String::from(" ")
            };
            let after: String = chars[(cursor_pos + 1).min(chars.len())..].iter().collect();

            if !before.is_empty() {
                spans.push(Span::raw(before));
            }
            spans.push(Span::styled(cursor_char, Style::new().black().on_white()));
            if !after.is_empty() {
                spans.push(Span::raw(after));
            }
            Line::from(spans)
        }
        InputMode::Normal => {
            if let Some((a, b)) = highlight_range {
                let start = a.min(b);
                let end = a.max(b);
                let before: String = chars[..start.min(chars.len())].iter().collect();
                let highlighted: String = chars[start.min(chars.len())..end.min(chars.len())]
                    .iter()
                    .collect();
                let after: String = chars[end.min(chars.len())..].iter().collect();

                let mut spans = Vec::new();
                if !before.is_empty() {
                    spans.push(Span::raw(before));
                }
                spans.push(Span::styled(
                    highlighted,
                    Style::new().bold().white().on_light_blue(),
                ));
                if !after.is_empty() {
                    spans.push(Span::raw(after));
                }
                Line::from(spans)
            } else {
                Line::raw(text.to_string())
            }
        }
    }
}
