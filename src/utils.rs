#![allow(clippy::pedantic)]
use crate::ui::App;
use agol::models::ArcGISSearchResults;
use std::collections::HashSet;

pub fn format_email(email: &str) -> &str {
    if email.eq_ignore_ascii_case("damian.sweet@cityoflonetree.com") {
        "Damian.Sweet@cityoflonetree.com"
    } else if email.eq_ignore_ascii_case("courtland.langley@cityoflonetree.com") {
        "courtland.langley@cityoflonetree.com"
    } else {
        email
    }
}

//TODO display feature layer info that has the most references

//TODO test how many results come from this
pub fn filter_layer_no_references(state: &mut App) -> Vec<&ArcGISSearchResults> {
    let mut no_reference_ids = Vec::new();
    for (k, v) in &state.agol.references.lookup {
        if v.is_empty() {
            no_reference_ids.push(k)
        }
    }

    state
        .agol
        .agol_content
        .iter()
        .filter(|c| {
            no_reference_ids.contains(&&c.id.clone()) && c.item_type != "Service Definition"
        })
        .collect()
}

//TODO call this from action not UI
pub fn get_layer_references(id: &str, app: &App) -> HashSet<ArcGISSearchResults> {
    if let Some(references) = app.agol.references.lookup.get(id) {
        references.clone()
    } else {
        HashSet::new()
    }
}

pub fn clear_highlight(app: &mut App) {
    app.state.user_input.highlight_range = None;
}

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

#[cfg(test)]
mod tests {
    // use super::*;
    // #[test]
    // fn test_helix_previous_word() {
    // }
}
