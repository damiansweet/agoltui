use crate::{
    UiState, filter_layer_no_references, load_all_content_from_file, read_last_sync, refresh_data,
    ui,
};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::Backend};
use std::collections::HashSet;
pub enum Action {
    SyncData,
    MoveSelectionDown,
    MoveSelectionUp,
    ZeroReferences,
    FilterByUsernameCli,
    SearchByKeyword,
    ListUsers,
    Reset,
    NoOp,
    Quit,
}

fn move_selection(current: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let cur = current.unwrap_or(0) as isize;
    let len_i = len as isize;
    Some(((cur + delta).rem_euclid(len_i)) as usize)
}

fn filter_by_username_cli(state: &mut UiState) {
    if let Some(email) = &state.cli_input.email {
        let filtered_list: Vec<agol::models::ArcGISSearchResults> = state
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.owner == format!("{email}_CityofLoneTree"))
            .cloned()
            .collect();

        state.agol_content = filtered_list;
    }
}

fn filter_by_username_widget(state: &mut UiState, username: String) {
    let filtered_list: Vec<agol::models::ArcGISSearchResults> = state
        .agol_content
        .iter()
        .filter(|agol_item| agol_item.owner == username)
        .cloned()
        .collect();

    state.agol_content = filtered_list;
}

fn search_by_keyword(state: &mut UiState, search_term: String) {
    let search_results: Vec<agol::models::ArcGISSearchResults> = state
        .agol_content
        .iter()
        .filter(|agol_item| {
            agol_item
                .title
                .to_lowercase()
                .contains(&search_term.to_lowercase())
        })
        .cloned()
        .collect();

    state.search_popup = true;
    state.agol_content = search_results;
}

fn all_usernames(state: &mut UiState) {
    let users: HashSet<String> = state
        .agol_content
        .iter()
        .map(|agol_item| agol_item.owner.clone())
        .collect();

    if !users.is_empty() {
        state.usernames.extend(users);
    }
}

fn reset_filters(state: &mut UiState) {
    let agol_content = load_all_content_from_file();

    match agol_content {
        Ok(content) => {
            state.agol_content = content;
            state.search_popup = false;
            state.usernames.clear();
        }
        //TODO call refresh data if Err
        Err(_) => {}
    }
}

pub fn handle_key(key: KeyCode) -> Action {
    let action = match key {
        KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionUp,
        KeyCode::Enter => Action::SyncData,
        KeyCode::Char('0') => Action::ZeroReferences,
        KeyCode::Char('f') => Action::FilterByUsernameCli,
        KeyCode::Char('s') => Action::SearchByKeyword,
        KeyCode::Char('u') => Action::ListUsers,
        KeyCode::Esc => Action::Reset,
        KeyCode::Char('q') => Action::Quit,
        _ => Action::NoOp,
    };

    action
}

pub fn handle_action(
    state: &mut UiState,
    terminal: &mut Terminal<impl Backend>,
    action: Action,
    client: &reqwest::blocking::Client,
    access_token: &agol::models::ArcGISAccessToken,
) {
    match action {
        Action::MoveSelectionDown => {
            let next = move_selection(state.selected, state.agol_content.len(), 1);
            state.selected = next;
            state.list_state.select(next);
        }
        Action::MoveSelectionUp => {
            let previous = move_selection(state.selected, state.agol_content.len(), -1);
            state.selected = previous;
            state.list_state.select(previous);
        }
        Action::SyncData => {
            state.loading = true;
            terminal
                .draw(|frame| {
                    ui(frame, state);
                })
                .expect("failed to draw loading screen");
            match refresh_data(client, access_token) {
                Ok(_) => {
                    let last_sync = read_last_sync();
                    state.last_synced = last_sync;
                    state.list_state.select(None);
                    state.loading = false;
                    state.agol_content = load_all_content_from_file().unwrap_or_default();
                }
                Err(_) => {}
            }
        }
        Action::ZeroReferences => {
            let list_content = filter_layer_no_references();
            state.agol_content = list_content;
            state.list_state.select(None);
        }
        // Action::FilterByUsername => {
        //     filter_by_username(
        //         state,
        //         String::from("Damian.Sweet@cityoflonetree.com_CityofLoneTree"),
        //     );
        // }
        Action::FilterByUsernameCli => {
            filter_by_username_cli(state);
        }
        Action::SearchByKeyword => {
            search_by_keyword(state, String::from("Boundary"));
        }
        Action::ListUsers => all_usernames(state),
        Action::Reset => {
            reset_filters(state);
        }
        Action::Quit => {
            state.running = false;
        }
        _ => {}
    }
}
