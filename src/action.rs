use crate::{
    UiState, filter_layer_no_references, load_all_content_from_file, read_last_sync, refresh_data,
    ui,
};
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::Backend};
pub enum Action {
    SyncData,
    MoveSelectionDown,
    MoveSelectionUp,
    ZeroReferences,
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

pub fn handle_key(key: KeyCode) -> Action {
    let action = match key {
        KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionUp,
        KeyCode::Enter => Action::SyncData,
        KeyCode::Char('0') => Action::ZeroReferences,
        KeyCode::Char('q') => Action::Quit,
        _ => Action::NoOp,
    };

    action
}

pub fn handle_action(
    state: &mut UiState,
    terminal: &mut Terminal<impl Backend>,
    len: usize,
    action: Action,
    client: &reqwest::blocking::Client,
    access_token: &agol::models::ArcGISAccessToken,
) {
    match action {
        Action::MoveSelectionDown => {
            let next = move_selection(state.selected, len, 1);
            state.selected = next;
            state.list_state.select(next);
        }
        Action::MoveSelectionUp => {
            let previous = move_selection(state.selected, len, -1);
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
        Action::Quit => {
            state.running = false;
        }
        //TODO create filter no references
        //TODO create reset action back to all fc
        _ => {}
    }
}
