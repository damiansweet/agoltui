use std::collections::HashMap;

use agol::models::ArcGISSearchResults;
use clap::Parser;

use crate::utils;
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position},
    widgets::{
        Block, Clear, List, ListDirection, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
};

#[derive(Debug)]
pub struct UiState {
    pub agol_content: Vec<ArcGISSearchResults>,
    pub selected: Option<usize>,
    pub list_state: ListState,
    pub last_synced: String,
    pub running: bool,
    pub loading: bool,
    pub search_popup: bool,
    pub username_popup: bool,
    pub user_input: UserInput,
    pub input_mode: InputMode,
    pub usernames: HashMap<String, u16>,
    pub username_state: TableState,
    pub cli_input: Args,
    pub errors: Option<Errors>,
    pub queries: Vec<String>,
}

#[derive(Debug)]
pub struct UserInput {
    pub input: String,
    pub character_index: usize,
    // pub input_mode: InputMode,
}

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
pub enum Errors {
    NoAccessToken,
    EmailNotFound,
    NoExistingData,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Email of the user to search
    #[arg(short, long)]
    pub email: Option<String>,

    /// Search term to filter results
    #[arg(short, long)]
    pub search: Option<String>,
}
pub fn init_state(cli_input: Args) -> UiState {
    let mut list_state = ListState::default();
    let selected = Some(0);
    list_state.select(selected);
    let last_synced = utils::read_last_sync();
    let running = true;
    let loading = false;
    let search_popup = false;
    let username_popup = false;
    let input_mode = InputMode::Normal;

    let cli_search_term = cli_input.email.clone();

    let user_input = UserInput {
        input: cli_search_term.unwrap_or_default(),
        character_index: 0,
    };

    let usernames = HashMap::new();

    let mut username_state = TableState::default();
    username_state.select(Some(0));

    let mut errors = None;

    let queries = Vec::new();

    let agol_content = if let Ok(agol_content) = utils::load_all_content_from_file() {
        agol_content
    } else {
        errors = Some(Errors::NoExistingData);
        Vec::new()
        // let _ = utils::refresh_data(&client, &access_token);
        // utils::load_all_content_from_file().expect("unable to read from all content json")
    };

    UiState {
        agol_content,
        selected,
        list_state,
        last_synced,
        running,
        loading,
        search_popup,
        username_popup,
        user_input,
        input_mode,
        cli_input,
        usernames,
        username_state,
        errors,
        queries,
    }
}

fn selected_item<'a>(
    state: &UiState,
    items: &'a [ArcGISSearchResults],
) -> Option<&'a ArcGISSearchResults> {
    state.selected.and_then(|i| items.get(i))
}

// ERROR WIDGETS

fn no_existing_data_widget() -> Paragraph<'static> {
    Paragraph::new("Welcome to AgolTUI :)\n Please press Enter to fetch AGOL content")
        .block(Block::bordered())
        .style(Style::new().red())
        .alignment(Alignment::Center)
}

fn no_access_token_error_widget() -> Paragraph<'static> {
    Paragraph::new("No Access Token Found")
        .block(Block::bordered().title("Error"))
        .style(Style::new().red())
        .alignment(Alignment::Center)
}

fn email_not_found_widget() -> Paragraph<'static> {
    Paragraph::new("No username found")
        .block(Block::bordered().title("Error"))
        .style(Style::new().red())
        .alignment(Alignment::Center)
}

// SUCCESS WIDGETS

fn loading_screen_widget() -> Paragraph<'static> {
    Paragraph::new("Loading data, please wait...")
        .block(Block::bordered())
        .style(Style::new().yellow())
        .alignment(Alignment::Center)
}

//TODO create widget that shows filter/search combos at bottom of screen

pub fn ui(frame: &mut Frame, state: &mut UiState) {
    match state.errors {
        Some(Errors::NoExistingData) => {
            let no_existing_data_widget = no_existing_data_widget();

            frame.render_widget(no_existing_data_widget, frame.area())
        }
        Some(Errors::NoAccessToken) => {
            let no_access_token_error_widget = no_access_token_error_widget();

            frame.render_widget(no_access_token_error_widget, frame.area())
        }
        Some(Errors::EmailNotFound) => {
            let email_not_found_widget = email_not_found_widget();
            frame.render_widget(email_not_found_widget, frame.area())
        }
        None => {
            if state.search_popup {
                let user_input = Paragraph::new(state.user_input.input.as_str())
                    .block(Block::bordered().title("Input"));

                let input_area = frame.area();
                frame.render_widget(Clear, frame.area());
                frame.render_widget(user_input, input_area);
                match state.input_mode {
                    InputMode::Normal => {}
                    InputMode::Editing => {
                        frame.set_cursor_position(Position::new(
                            input_area.x + state.user_input.character_index as u16 + 1,
                            input_area.y + 1,
                        ));
                    }
                }
            } else {
                if state.loading {
                    let loading_screen_widget = loading_screen_widget();
                    frame.render_widget(loading_screen_widget, frame.area())
                } else if state.usernames.len() > 0 {
                    let rows: Vec<Row> = state
                        .usernames
                        .iter()
                        .map(|(k, v)| Row::new(vec![k.to_string(), v.to_string()]))
                        .collect();

                    let widths = [Constraint::Length(60), Constraint::Length(20)];
                    let username_widget = Table::new(rows, widths)
                        .column_spacing(1)
                        .style(Style::new().blue())
                        .highlight_symbol(">>")
                        .header(Row::new(vec!["Username", "# of Items"]))
                        .block(Block::new().title("Usernames Table"));
                    frame.render_stateful_widget(
                        username_widget,
                        frame.area(),
                        &mut state.username_state,
                    )
                } else {
                    // let area = frame.area();
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![
                            Constraint::Percentage(40),
                            Constraint::Percentage(20),
                            Constraint::Percentage(40),
                        ])
                        .split(frame.area());

                    let all_content_ids: Vec<ListItem> = state
                        .agol_content
                        .iter()
                        .map(|item| ListItem::new(item.id.clone()))
                        .collect();

                    let widget_left = List::new(all_content_ids)
                        .block(Block::bordered().title("All AGOL Content List"))
                        .style(Style::new().white())
                        .highlight_style(Style::new().italic())
                        .highlight_symbol(">>")
                        .repeat_highlight_symbol(true)
                        .direction(ListDirection::TopToBottom);

                    // let content_count = all_agol_content.len();
                    let selected_title = selected_item(state, &state.agol_content)
                        .map(|item| item.title.as_str())
                        .unwrap_or_default();

                    let selected_owner = selected_item(state, &state.agol_content)
                        .map(|item| item.owner.as_str())
                        .unwrap_or_default();
                    let last_sync = &state.last_synced.clone();

                    let queries = &state.queries.join(" && ");

                    let layer_info_text = format!(
                        "Title: {selected_title}\nOwner: {selected_owner}\nReferences Last Synced: {last_sync}\n<j>/<Down> Navigate Down | <k>/<Up> Navigate Up\n<Enter> to refresh data | <f> to filter by username | <0> zero references\nCurrent Query: {queries}"
                    );

                    let widget_center = Paragraph::new(layer_info_text)
                        .wrap(Wrap { trim: true })
                        .block(Block::bordered().title("Layer Info"))
                        .style(Style::new().white())
                        .alignment(Alignment::Center);

                    let widget_right = if let Some(selected_id) =
                        selected_item(state, &state.agol_content).map(|item| item.id.as_str())
                    {
                        let references = utils::get_layer_references(selected_id);
                        if references.len() >= 1 {
                            List::new(references)
                                .block(Block::bordered().title("References"))
                                .style(Style::new().blue())
                                .direction(ListDirection::TopToBottom)
                        } else {
                            List::default()
                                .block(Block::bordered().title("No References"))
                                .style(Style::new().red())
                                .direction(ListDirection::TopToBottom)
                        }
                    } else {
                        List::default()
                    };

                    // if state.search_popup {
                    // } else {
                    frame.render_stateful_widget(widget_left, layout[0], &mut state.list_state);

                    frame.render_widget(widget_center, layout[1]);

                    frame.render_widget(widget_right, layout[2]);
                    // }
                }
            }
        }
    }
}
