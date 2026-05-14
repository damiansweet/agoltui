use crate::ui::{App, InputMode, SearchType, State};
use crate::utils::{
    clear_highlight, extract_usernames, filter_layer_no_references, format_email,
    get_layer_references, helix_next_word, helix_previous_word,
};

use agol::models::ArcGISSearchResults;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    MoveSelectionDown,
    MoveSelectionUp,
    MoveReferenceDown,
    MoveReferenceUp,
    MoveBrokenConnectionDown,
    MoveBrokenConnectionUp,
    SwitchFocus,
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
    HelixPreviousWord,
    HelixNextWord,
    FocusBrokenConnections,
    GoBack,
}

fn move_selection(current: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let cur = current.unwrap_or(0) as isize;
    let len_i = len as isize;
    Some(((cur + delta).rem_euclid(len_i)) as usize)
}

fn move_search_cursor_left(app: &mut App) {
    let cursor_moved_left = app.state.user_input.character_index.saturating_sub(1);
    app.state.user_input.character_index = clamp_cursor(app, cursor_moved_left);
}

fn move_search_cursor_right(app: &mut App) {
    let cursor_moved_right = app.state.user_input.character_index.saturating_add(1);
    app.state.user_input.character_index = clamp_cursor(app, cursor_moved_right);
}

fn enter_char(app: &mut App, char: char) {
    let index = byte_index(app);
    app.state.user_input.input.insert(index, char);
    move_search_cursor_right(app);
}

fn byte_index(app: &App) -> usize {
    app.state
        .user_input
        .input
        .char_indices()
        .map(|(i, _)| i)
        .nth(app.state.user_input.character_index)
        .unwrap_or(app.state.user_input.input.len())
}

fn delete_char(app: &mut App) {
    let is_not_cursor_leftmost = app.state.user_input.character_index != 0;
    if is_not_cursor_leftmost {
        let current_index = app.state.user_input.character_index;
        let from_left_to_current_index = current_index - 1;

        let before_char_to_delete = app
            .state
            .user_input
            .input
            .chars()
            .take(from_left_to_current_index);

        let after_char_to_delete = app.state.user_input.input.chars().skip(current_index);

        app.state.user_input.input = before_char_to_delete.chain(after_char_to_delete).collect();
        move_search_cursor_left(app);
    }
}

fn clamp_cursor(app: &App, new_cursor_pos: usize) -> usize {
    new_cursor_pos.clamp(0, app.state.user_input.input.chars().count())
}

fn filter_by_username_cli(app: &mut App) {
    if let Some(email) = &app.state.cli_input.email {
        let email = format_email(email);
        let filtered_list: Vec<ArcGISSearchResults> = app
            .agol
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.owner == email)
            .cloned()
            .collect();

        app.agol.agol_content = filtered_list;
        if !app.state.queries.contains(&format!("Username == {email}")) {
            app.state.queries.push(format!("Username == {email}"))
        };
    }
}

fn search_by_username(app: &mut App) {
    // let username = {
    //     format!(
    //         "{}_{}",
    //         format_email(state.user_input.input.as_str()),
    //         state.org_info.url_key
    //     )
    // };
    let username = app.state.user_input.input.clone();
    let filtered_list: Vec<ArcGISSearchResults> = app
        .agol
        .agol_content
        .iter()
        .filter(|agol_item| agol_item.owner == username)
        .cloned()
        .collect();

    app.agol.agol_content = filtered_list;
    app.state.search_popup = false;
    if !app
        .state
        .queries
        .contains(&format!("Owner/Username == '{username}'"))
    {
        app.state
            .queries
            .push(format!("Owner/Username == '{username}'"))
    };
}

fn search_by_keyword(app: &mut App) {
    let query = &app.state.user_input.input.to_lowercase();

    if query.len() >= 3 && query.len() <= 50 {
        let search_results: Vec<ArcGISSearchResults> = app
            .agol
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.title.to_lowercase().contains(query))
            .cloned()
            .collect();

        app.state.search_popup = false;
        if !app
            .state
            .queries
            .contains(&format!("title ILIKE '%{query}%'"))
        {
            app.state.queries.push(format!("Title ILIKE '%{query}%'"))
        };
        app.agol.agol_content = search_results;
        if app.agol.agol_content.is_empty() {
            app.state.agol_content_widget_state.select(None);
        } else {
            app.state.agol_content_widget_state.select(Some(0));
        }
    } else {
        app.state.errors = Some(crate::ui::Errors::InvalidUserInput);
    }
}

fn search_by_item_id(app: &mut App) {
    let query = &app.state.user_input.input;

    if query.len() >= 3 && query.len() <= 50 {
        let search_results: Vec<ArcGISSearchResults> = app
            .agol
            .agol_content
            .iter()
            .filter(|agol_item| agol_item.id == *query)
            .cloned()
            .collect();

        app.state.search_popup = false;
        if !app.state.queries.contains(&format!("id == '{query}'")) {
            app.state.queries.push(format!("id == '{query}'"))
        };
        app.agol.agol_content = search_results;
        if app.agol.agol_content.is_empty() {
            app.state.agol_content_widget_state.select(None);
        } else {
            app.state.agol_content_widget_state.select(Some(0));
        }
    } else {
        app.state.errors = Some(crate::ui::Errors::InvalidUserInput);
    }
}

fn flip_input_mode(app: &mut App) {
    match app.state.input_mode {
        InputMode::Normal => app.state.input_mode = InputMode::Editing,
        InputMode::Editing => app.state.input_mode = InputMode::Normal,
    };
}

fn set_search_type(app: &mut App, search_type: SearchType) {
    match search_type {
        SearchType::Title => app.state.search_type = SearchType::Title,
        SearchType::Owner => app.state.search_type = SearchType::Owner,
        SearchType::Id => app.state.search_type = SearchType::Id,
    }
}

fn launch_search(app: &mut App) {
    app.state.search_popup = true;
    app.state.input_mode = InputMode::Editing;
}

fn all_usernames(app: &mut App) {
    app.agol.agol_content.iter().for_each(|agol_item| {
        app.state
            .usernames
            .entry(agol_item.owner.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    });
}

async fn reset_filters(app: &mut App) {
    app.agol.agol_content = app.agol.cached_agol_content.clone();
    app.state.agol_content_widget_state.select(Some(0));
    app.state.user_input.character_index = 0;
    app.state.search_popup = false;
    app.state.usernames.clear();
    app.state.user_input.input.clear();
    app.state.queries.clear();
    app.state.errors = None;

    // dbg!(&state);
}

//TODO for broken connections list what the item title is that is broken not just web map/app
//
//
//
pub fn handle_key(state: &State, key: KeyEvent) -> Action {
    match state.input_mode {
        InputMode::Normal => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                if state.focused_widget == crate::ui::FocusedWidget::TopList {
                    Action::MoveSelectionDown
                } else if state.focused_widget == crate::ui::FocusedWidget::BottomTable {
                    Action::MoveReferenceDown
                } else {
                    Action::MoveBrokenConnectionDown
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                if state.focused_widget == crate::ui::FocusedWidget::TopList {
                    Action::MoveSelectionUp
                } else if state.focused_widget == crate::ui::FocusedWidget::BottomTable {
                    Action::MoveReferenceUp
                } else {
                    Action::MoveBrokenConnectionUp
                }
            }
            (KeyModifiers::NONE, KeyCode::Enter) if state.search_popup => {
                Action::UserInputSubmitQuery
            }
            (KeyModifiers::NONE, KeyCode::Char('0')) => Action::ZeroReferences,
            (KeyModifiers::NONE, KeyCode::Char('f')) => Action::FilterByUsernameCli,
            //TODO if pressing s clear user input and then launch search
            (KeyModifiers::NONE, KeyCode::Char('s')) | (KeyModifiers::NONE, KeyCode::Char('i')) => {
                Action::SearchByKeyword
            }
            (KeyModifiers::NONE, KeyCode::Char('u')) => Action::ListUsers,
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if state.focused_widget == crate::ui::FocusedWidget::BrokenConnections {
                    Action::GoBack
                } else {
                    Action::Reset
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) => Action::Quit,
            (KeyModifiers::NONE, KeyCode::Tab) => Action::SwitchFocus,
            (KeyModifiers::NONE, KeyCode::Char('b')) => Action::HelixPreviousWord,
            (KeyModifiers::SHIFT, KeyCode::Char('B')) => Action::FocusBrokenConnections,
            (KeyModifiers::NONE, KeyCode::Char('w')) => Action::HelixNextWord,
            _ => Action::NoOp,
        },
        InputMode::Editing => match (key.modifiers, key.code) {
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

pub async fn handle_action(app: &mut App, action: Action) {
    match action {
        Action::MoveSelectionDown => {
            let next = move_selection(
                app.state.agol_content_widget_state.selected(),
                app.agol.agol_content.len(),
                1,
            );
            app.state.agol_content_widget_state.select(next);
        }
        Action::MoveSelectionUp => {
            let previous = move_selection(
                app.state.agol_content_widget_state.selected(),
                app.agol.agol_content.len(),
                -1,
            );
            app.state.agol_content_widget_state.select(previous);
        }
        Action::MoveReferenceDown => {
            if let Some(selected_id) = app
                .state
                .agol_content_widget_state
                .selected()
                .and_then(|i| app.agol.agol_content.get(i))
                .map(|item| item.id.as_str())
            {
                let references = get_layer_references(selected_id, app);
                let len = references.len();
                if len > 0 {
                    let current = app.state.reference_table_state.selected().unwrap_or(0);
                    let next = ((current as isize + 1).rem_euclid(len as isize)) as usize;
                    app.state.reference_table_state.select(Some(next));
                }
            }
        }
        Action::MoveReferenceUp => {
            if let Some(selected_id) = app
                .state
                .agol_content_widget_state
                .selected()
                .and_then(|i| app.agol.agol_content.get(i))
                .map(|item| item.id.as_str())
            {
                let references = get_layer_references(selected_id, app);
                let len = references.len();
                if len > 0 {
                    let current = app.state.reference_table_state.selected().unwrap_or(0);
                    let prev = ((current as isize - 1).rem_euclid(len as isize)) as usize;
                    app.state.reference_table_state.select(Some(prev));
                }
            }
        }
        Action::MoveBrokenConnectionDown => {
            let len = app.agol.references.broken_connections.len();
            if len > 0 {
                let current = app.state.broken_connections_state.selected().unwrap_or(0);
                let next = ((current as isize + 1).rem_euclid(len as isize)) as usize;
                app.state.broken_connections_state.select(Some(next));
            }
        }
        Action::MoveBrokenConnectionUp => {
            let len = app.agol.references.broken_connections.len();
            if len > 0 {
                let current = app.state.broken_connections_state.selected().unwrap_or(0);
                let prev = ((current as isize - 1).rem_euclid(len as isize)) as usize;
                app.state.broken_connections_state.select(Some(prev));
            }
        }
        Action::FocusBrokenConnections => {
            app.state.focused_widget = crate::ui::FocusedWidget::BrokenConnections;
        }
        Action::GoBack => {
            app.state.focused_widget = crate::ui::FocusedWidget::TopList;
        }
        Action::SwitchFocus => match app.state.focused_widget {
            crate::ui::FocusedWidget::TopList => {
                app.state.focused_widget = crate::ui::FocusedWidget::BottomTable;
            }
            crate::ui::FocusedWidget::BottomTable => {
                app.state.focused_widget = crate::ui::FocusedWidget::TopList;
            }
            crate::ui::FocusedWidget::BrokenConnections => {}
        },
        Action::ZeroReferences => {
            let list_content = filter_layer_no_references(app)
                .into_iter()
                .cloned()
                .collect();
            app.agol.agol_content = list_content;
            if app.agol.agol_content.is_empty() {
                app.state.agol_content_widget_state.select(None);
            } else {
                app.state.agol_content_widget_state.select(Some(0))
            }
            if !app.state.queries.contains(&String::from("Zero References")) {
                app.state.queries.push(String::from("Zero References"))
            };
        }
        // Action::FilterByUsername => {
        //     filter_by_username(
        //         app,
        //         String::from("Damian.Sweet@cityoflonetree.com_CityofLoneTree"),
        //     );
        // }
        Action::FilterByUsernameCli => {
            filter_by_username_cli(app);
        }
        Action::SearchByKeyword => {
            launch_search(app);
            // search_by_keyword(app);
        }
        Action::UserInputEnterChar(char) => {
            enter_char(app, char);
            clear_highlight(app);
        }
        Action::UserInputDeleteChar => {
            delete_char(app);
            clear_highlight(app);
        }
        Action::UserInputFlipInputMode => {
            flip_input_mode(app);
            clear_highlight(app);
        }
        Action::UserInputSearchTerm
            if app.state.search_type == SearchType::Owner
                || app.state.search_type == SearchType::Id =>
        {
            set_search_type(app, SearchType::Title);
        }
        Action::UserInputSearchUsername
            if app.state.search_type == SearchType::Title
                || app.state.search_type == SearchType::Id =>
        {
            set_search_type(app, SearchType::Owner);
        }
        Action::UserInputSearchId
            if app.state.search_type == SearchType::Title
                || app.state.search_type == SearchType::Owner =>
        {
            set_search_type(app, SearchType::Id);
        }
        Action::UserInputSubmitQuery if app.state.search_type == SearchType::Title => {
            search_by_keyword(app);
            flip_input_mode(app);
            clear_highlight(app);
        }
        Action::UserInputSubmitQuery if app.state.search_type == SearchType::Owner => {
            search_by_username(app);
            flip_input_mode(app);
            clear_highlight(app);
        }
        Action::UserInputSubmitQuery if app.state.search_type == SearchType::Id => {
            search_by_item_id(app);
            flip_input_mode(app);
            clear_highlight(app);
        }

        Action::ListUsers => {
            all_usernames(app);
            // TODO create action/widget for valid usernames to display below search
            // panic!(
            //     "total users: {:?}",
            //     extract_usernames(&app.agol.users).len()
            // );
        }
        Action::Reset => {
            reset_filters(app).await;
            if app.state.focused_widget == crate::ui::FocusedWidget::BrokenConnections {
                app.state.focused_widget = crate::ui::FocusedWidget::TopList;
            }
        }
        Action::Quit => {
            app.state.running = false;
        }
        Action::HelixPreviousWord => {
            helix_previous_word(app);
        }
        Action::HelixNextWord => {
            helix_next_word(app);
        }
        Action::NoOp => {}
        _ => {}
    }
}
