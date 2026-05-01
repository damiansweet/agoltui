use crate::ui::{InputMode, SearchType, UiState};
use crate::utils::{filter_layer_no_references, format_email};
use std::sync::Arc;

use agol::models::{ArcGISAccessToken, ArcGISSearchResults};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    UserInputSearchTerm,
    UserInputSearchUsername,
    UserInputSearchId,
    UserInputEnterChar(char),
    UserInputDeleteChar,
    UserInputSubmitQuery,
    UserInputFlipInputMode,
}

//TODO add copy selected id to system clipboard with full url for easy navigation

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

fn clamp_cursor(state: &UiState, new_cursor_pos: usize) -> usize {
    new_cursor_pos.clamp(0, state.user_input.input.chars().count())
}

fn filter_by_username_cli(state: &mut UiState) {
    if let Some(email) = &state.cli_input.email {
        //TODO verify email is in org
        let email = format_email(email);
        let filtered_list: Vec<ArcGISSearchResults> = state
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.owner == format!("{email}_CityofLoneTree"))
            .cloned()
            .collect();

        state.agol_content = filtered_list;
        if !state.queries.contains(&format!("Username == {email}")) {
            state.queries.push(format!("Username == {email}"))
        };
    }
}

fn search_by_username(state: &mut UiState) {
    let username = {
        format!(
            "{}_{}",
            format_email(state.user_input.input.as_str()),
            state.org_info.url_key
        )
    };
    let filtered_list: Vec<ArcGISSearchResults> = state
        .agol_content
        .iter()
        .filter(|agol_item| agol_item.owner == username)
        .cloned()
        .collect();

    state.agol_content = filtered_list;
    state.search_popup = false;
    if !state
        .queries
        .contains(&format!("Owner/Username == '{username}'"))
    {
        state
            .queries
            .push(format!("Owner/Username == '{username}'"))
    };
}

fn search_by_keyword(state: &mut UiState) {
    let query = &state.user_input.input.to_lowercase();

    if query.len() >= 3 && query.len() <= 50 {
        let search_results: Vec<ArcGISSearchResults> = state
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.title.to_lowercase().contains(query))
            .cloned()
            .collect();

        state.search_popup = false;
        if !state.queries.contains(&format!("title ILIKE '%{query}%'")) {
            state.queries.push(format!("Title ILIKE '%{query}%'"))
        };
        state.agol_content = search_results;
        state.selected = Some(0);
        state.list_state.select_first();
    } else {
        state.errors = Some(crate::ui::Errors::InvalidUserInput);
    }
}

fn search_by_item_id(state: &mut UiState) {
    let query = &state.user_input.input;

    if query.len() >= 3 && query.len() <= 50 {
        let search_results: Vec<ArcGISSearchResults> = state
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.id == *query)
            .cloned()
            .collect();

        state.search_popup = false;
        if !state.queries.contains(&format!("id == '{query}'")) {
            state.queries.push(format!("id == '{query}'"))
        };
        state.agol_content = search_results;
        state.selected = Some(0);
        state.list_state.select_first();
    } else {
        state.errors = Some(crate::ui::Errors::InvalidUserInput);
    }
}

fn flip_input_mode(state: &mut UiState) {
    match state.input_mode {
        InputMode::Normal => state.input_mode = InputMode::Editing,
        InputMode::Editing => state.input_mode = InputMode::Normal,
    };
}

fn set_search_type(state: &mut UiState, search_type: SearchType) {
    match search_type {
        SearchType::Title => state.search_type = SearchType::Title,
        SearchType::Owner => state.search_type = SearchType::Owner,
        SearchType::Id => state.search_type = SearchType::Id,
    }
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

async fn reset_filters(
    state: &mut UiState,
    client: &reqwest::Client,
    access_token: ArcGISAccessToken,
) {
    // dbg!(&state);
    //TODO create a filtered UiState struct field and reset to all content when called
    let client = Arc::new(client.clone());
    let access_token = Arc::new(access_token);

    if let Ok(agol_content) = agol::fetch_all_agol_content(
        client.clone(),
        access_token,
        state.agol_total_count,
        &state.org_info.org_id,
    )
    .await
    {
        state.agol_content = agol_content;
        state.selected = Some(0);
        state.list_state.select_first();
        state.user_input.character_index = 0;
        state.search_popup = false;
        state.usernames.clear();
        state.queries.clear();
        state.errors = None;
    }

    // dbg!(&state);
}

pub fn handle_key(state: &UiState, key: KeyEvent) -> Action {
    match state.input_mode {
        InputMode::Normal => match (key.modifiers, key.code) {
            //TODO fix rest of keybinds with adding modifiers
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                Action::MoveSelectionDown
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                Action::MoveSelectionUp
            }
            (KeyModifiers::NONE, KeyCode::Enter) => Action::SyncData,
            (KeyModifiers::NONE, KeyCode::Char('0')) => Action::ZeroReferences,
            (KeyModifiers::NONE, KeyCode::Char('f')) => Action::FilterByUsernameCli,
            (KeyModifiers::NONE, KeyCode::Char('s')) => Action::SearchByKeyword,
            (KeyModifiers::NONE, KeyCode::Char('u')) => Action::ListUsers,
            (KeyModifiers::NONE, KeyCode::Esc) => Action::Reset,
            (KeyModifiers::NONE, KeyCode::Char('q')) => Action::Quit,
            _ => Action::NoOp,
        },
        InputMode::Editing => match (key.modifiers, key.code) {
            //TODO change below to be ctrl+ 1/2
            (KeyModifiers::NONE, KeyCode::F(1)) => Action::UserInputSearchTerm,
            (KeyModifiers::NONE, KeyCode::F(2)) => Action::UserInputSearchUsername,
            (KeyModifiers::NONE, KeyCode::F(3)) => Action::UserInputSearchId,
            (KeyModifiers::NONE, KeyCode::Char(typed_char))
            | (KeyModifiers::SHIFT, KeyCode::Char(typed_char)) => {
                Action::UserInputEnterChar(typed_char)
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => Action::UserInputDeleteChar,
            (KeyModifiers::NONE, KeyCode::Esc) => Action::UserInputFlipInputMode,
            (KeyModifiers::NONE, KeyCode::Enter) => Action::UserInputSubmitQuery,
            _ => Action::NoOp,
        },
    }
}

pub async fn handle_action(
    state: &mut UiState,
    action: Action,
    client: &reqwest::Client,
    access_token: &ArcGISAccessToken,
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
            // state.loading = true;
            // state.errors = None;
            // terminal
            //     .draw(|frame| {
            //         ui(frame, state);
            //     })
            //     .expect("failed to draw loading screen");
            // match refresh_data(client, access_token).await {
            //     Ok(_) => {
            //         let last_sync = read_last_sync();
            //         state.last_synced = last_sync;
            //         state.list_state.select(Some(0));
            //         state.loading = false;
            //         state.agol_content = load_all_content_from_file().unwrap_or_default();
            //     }
            //     Err(_) => {}
            // }
        }
        Action::ZeroReferences => {
            let list_content = filter_layer_no_references(state)
                .into_iter()
                .cloned()
                .collect();
            state.agol_content = list_content;
            state.selected = Some(0);
            state.list_state.select_first();
            if !state.queries.contains(&String::from("Zero References")) {
                state.queries.push(String::from("Zero References"))
            };
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
        Action::UserInputSearchTerm
            if state.search_type == SearchType::Owner || state.search_type == SearchType::Id =>
        {
            set_search_type(state, SearchType::Title);
        }
        Action::UserInputSearchUsername
            if state.search_type == SearchType::Title || state.search_type == SearchType::Id =>
        {
            set_search_type(state, SearchType::Owner);
        }
        Action::UserInputSearchId
            if state.search_type == SearchType::Title || state.search_type == SearchType::Owner =>
        {
            set_search_type(state, SearchType::Id);
        }
        Action::UserInputSubmitQuery if state.search_type == SearchType::Title => {
            search_by_keyword(state);
            flip_input_mode(state);
        }
        Action::UserInputSubmitQuery if state.search_type == SearchType::Owner => {
            search_by_username(state);
            flip_input_mode(state);
        }
        Action::UserInputSubmitQuery if state.search_type == SearchType::Id => {
            search_by_item_id(state);
            flip_input_mode(state);
        }

        Action::ListUsers => all_usernames(state),
        Action::Reset => {
            reset_filters(state, client, access_token.clone()).await;
        }
        Action::Quit => {
            state.running = false;
        }
        _ => {}
    }
}
