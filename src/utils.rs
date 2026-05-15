use crate::models::{
    App, Args, CliArgsFilter, Errors, FocusedWidget, InputMode, SearchType, State, UserInput,
};
use agol::models::{ArcGISSearchResults, Users};
use ratatui::widgets::{ListState, TableState};
use std::collections::{HashMap, HashSet};

pub fn filter_layer_no_references<'a>(state: &'a mut App) {
    let mut no_reference_ids = Vec::new();
    for (k, v) in &state.agol.references.lookup {
        if v.is_empty() {
            no_reference_ids.push(k)
        }
    }

    let existing_state = state.agol.agol_content.clone();
    state.agol.agol_content = existing_state
        .iter()
        .filter(|c| {
            no_reference_ids.contains(&&c.id.clone()) && c.item_type != "Service Definition"
        })
        .copied()
        .collect();
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

pub fn default_app_state() -> State {
    State {
        agol_content_widget_state: ListState::default().with_selected(Some(0)),
        reference_table_state: TableState::default().with_selected(Some(0)),
        broken_connections_state: TableState::default().with_selected(Some(0)),
        username_state: TableState::default().with_selected(Some(0)),
        focused_widget: FocusedWidget::default(),
        user_input: UserInput::default(),
        search_type: SearchType::default(),
        input_mode: InputMode::default(),
        items_per_username: HashMap::default(),
        cli_input: Args {
            email: None,
            search: None,
        },
        errors: None,
        queries: Vec::default(),
        running: true,
        references_loading: true,
        users_loading: true,
        search_popup: false,
    }
}

pub fn filter_cli_args<'a>(
    agol_items: &'a [ArcGISSearchResults],
    email: Option<&str>,
    search_term: Option<&str>,
    filter_type: CliArgsFilter,
) -> Vec<&'a ArcGISSearchResults> {
    match filter_type {
        CliArgsFilter::Both => agol_items
            .iter()
            .filter(|i| i.owner == email.unwrap() && i.title.contains(search_term.unwrap()))
            .collect(),
        CliArgsFilter::Email => agol_items
            .iter()
            .filter(|i| i.owner == email.unwrap())
            .collect(),
        CliArgsFilter::SearchTerm => agol_items
            .iter()
            .filter(|i| i.title.contains(search_term.unwrap()))
            .collect(),
        CliArgsFilter::None => agol_items.iter().collect(),
    }
}

//TODO create test for  default_app_state

#[cfg(test)]
mod tests {
    // use super::*;
    // #[test]
    // fn test_helix_previous_word() {
    // }
}
