use crate::ui::{InputMode, UiState, ui};
use crate::utils::{
    filter_layer_no_references, format_email, load_all_content_from_file, read_last_sync,
    refresh_data,
};

use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::Backend};

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
    UserInputEnterChar(char),
    UserInputDeleteChar,
    UserInputMoveCursorLeft,
    UserInputMoveCursorRight,
    UserInputSubmitQuery,
    UserInputFlipInputMode,
}

fn move_selection(current: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let cur = current.unwrap_or(0) as isize;
    let len_i = len as isize;
    Some(((cur + delta).rem_euclid(len_i)) as usize)
}

fn move_search_cursor_left(state: &mut UiState) {
    let cursor_moved_left = state.user_input.character_index.saturating_sub(1);
    state.user_input.character_index = clamp_cursor(state, cursor_moved_left);
}

fn move_search_cursor_right(state: &mut UiState) {
    let cursor_moved_right = state.user_input.character_index.saturating_add(1);
    state.user_input.character_index = clamp_cursor(state, cursor_moved_right);
}

fn enter_char(state: &mut UiState, char: char) {
    let index = byte_index(state);
    state.user_input.input.insert(index, char);
    move_search_cursor_right(state);
}

fn byte_index(state: &UiState) -> usize {
    state
        .user_input
        .input
        .char_indices()
        .map(|(i, _)| i)
        .nth(state.user_input.character_index)
        .unwrap_or(state.user_input.input.len())
}

fn delete_char(state: &mut UiState) {
    let is_not_cursor_leftmost = state.user_input.character_index != 0;
    if is_not_cursor_leftmost {
        let current_index = state.user_input.character_index;
        let from_left_to_current_index = current_index - 1;

        let before_char_to_delete = state
            .user_input
            .input
            .chars()
            .take(from_left_to_current_index);

        let after_char_to_delete = state.user_input.input.chars().skip(current_index);

        state.user_input.input = before_char_to_delete.chain(after_char_to_delete).collect();
        move_search_cursor_left(state);
    }
}

fn submit_user_input_search(state: &mut UiState) {
    search_by_keyword(state);
    state.input_mode = InputMode::Normal;
}

fn clamp_cursor(state: &UiState, new_cursor_pos: usize) -> usize {
    new_cursor_pos.clamp(0, state.user_input.input.chars().count())
}

fn reset_cursor(state: &mut UiState) {
    state.user_input.character_index = 0;
}

fn filter_by_username_cli(state: &mut UiState) {
    if let Some(email) = &state.cli_input.email {
        //TODO verify email is in org
        let email = format_email(email);
        let filtered_list: Vec<agol::models::ArcGISSearchResults> = state
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.owner == format!("{email}_CityofLoneTree"))
            .cloned()
            .collect();

        state.agol_content = filtered_list;
        state.queries.push(format!("Username == {email}"));
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

fn search_by_keyword(state: &mut UiState) {
    let query = &state.user_input.input.to_lowercase();
    let search_results: Vec<agol::models::ArcGISSearchResults> = state
        .agol_content
        .iter()
        .filter(|agol_item| agol_item.title.to_lowercase().contains(query))
        .cloned()
        .collect();

    state.search_popup = false;
    state.queries.push(format!("Title ILIKE '%{query}%'"));
    state.agol_content = search_results;
}

fn flip_input_mode(state: &mut UiState) {
    match state.input_mode {
        InputMode::Normal => state.input_mode = InputMode::Editing,
        InputMode::Editing => state.input_mode = InputMode::Normal,
    };
}

fn launch_search(state: &mut UiState) {
    state.search_popup = true;
    state.input_mode = InputMode::Editing;
}

fn all_usernames(state: &mut UiState) {
    state.agol_content.iter().for_each(|agol_item| {
        state
            .usernames
            .entry(agol_item.owner.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    });
}

fn reset_filters(state: &mut UiState) {
    let agol_content = load_all_content_from_file();

    match agol_content {
        Ok(content) => {
            state.agol_content = content;
            state.search_popup = false;
            state.usernames.clear();
            state.queries.clear();
        }
        //TODO call refresh data if Err
        Err(_) => {}
    }
}

pub fn handle_key(state: &UiState, key: KeyCode) -> Action {
    let action = match state.input_mode {
        InputMode::Normal => match key {
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
        },
        InputMode::Editing => match key {
            KeyCode::Char(typed_char) => Action::UserInputEnterChar(typed_char),
            KeyCode::Backspace => Action::UserInputDeleteChar,
            KeyCode::Esc => Action::UserInputFlipInputMode,
            KeyCode::Enter => Action::UserInputSubmitQuery,
            _ => Action::NoOp,
        },
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
            state.errors = None;
            terminal
                .draw(|frame| {
                    ui(frame, state);
                })
                .expect("failed to draw loading screen");
            match refresh_data(client, access_token) {
                Ok(_) => {
                    let last_sync = read_last_sync();
                    state.last_synced = last_sync;
                    state.list_state.select(Some(0));
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
            state.queries.push(String::from("Zero References"));
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
            launch_search(state);
            // search_by_keyword(state);
        }
        Action::UserInputEnterChar(char) => {
            enter_char(state, char);
        }
        Action::UserInputDeleteChar => {
            delete_char(state);
        }
        Action::UserInputFlipInputMode => flip_input_mode(state),
        Action::UserInputSubmitQuery => {
            search_by_keyword(state);
            flip_input_mode(state);
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
