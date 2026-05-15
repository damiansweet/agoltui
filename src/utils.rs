use crate::models::{App, State};
use agol::models::{ArcGISSearchResults, Users};
use std::collections::HashSet;

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

pub fn clear_user_input(app_state: &mut State) {
    app_state.user_input.input.clear();
}

pub fn disable_search_popup(app_state: &mut State) {
    app_state.search_popup = false;
}

pub fn extract_usernames(users: &[Users]) -> Vec<&str> {
    users.iter().map(|u| u.username.as_str()).collect()
}

#[cfg(test)]
mod tests {
    // use super::*;
    // #[test]
    // fn test_helix_previous_word() {
    // }
}
