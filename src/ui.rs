use std::collections::HashMap;

use agol::models::ArcGISSearchResults;
use clap::Parser;

use crate::utils;
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
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
    pub usernames: HashMap<String, u16>,
    pub username_state: TableState,
    pub cli_input: Args,
    pub errors: Errors,
}

#[derive(Debug)]
pub struct UserInput {
    pub input: String,
    pub character_index: usize,
    pub input_mode: InputMode,
}

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Default)]
pub enum Errors {
    NoAccessToken,
    EmailNotFound,
    NoExistingData,
    #[default]
    None,
}

#[derive(Parser, Debug)]
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
    let user_input = UserInput {
        input: String::new(),
        input_mode: InputMode::Normal,
        character_index: 0,
    };

    let usernames = HashMap::new();

    let mut username_state = TableState::default();
    username_state.select(Some(0));

    let mut cli_input = cli_input;
    utils::format_email(&mut cli_input);

    let mut errors = Errors::default();

    let agol_content = if let Ok(agol_content) = utils::load_all_content_from_file() {
        agol_content
    } else {
        errors = Errors::NoExistingData;
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
        cli_input,
        usernames,
        username_state,
        errors,
    }
}

fn selected_item<'a>(
    state: &UiState,
    items: &'a [ArcGISSearchResults],
) -> Option<&'a ArcGISSearchResults> {
    state.selected.and_then(|i| items.get(i))
}

pub fn ui(frame: &mut Frame, state: &mut UiState) {
    match state.errors {
        Errors::NoExistingData => {
            let error_widget =
                Paragraph::new("No Existing Data Found\n Please press Enter to fetch AGOL content")
                    .block(Block::bordered().title("Error"))
                    .style(Style::new().red())
                    .alignment(Alignment::Center);

            frame.render_widget(error_widget, frame.area())
        }
        Errors::NoAccessToken => {
            let error_widget = Paragraph::new("No Access Token Found")
                .block(Block::bordered().title("Error"))
                .style(Style::new().red())
                .alignment(Alignment::Center);

            frame.render_widget(error_widget, frame.area())
        }
        Errors::EmailNotFound => {
            let error_widget = Paragraph::new("No username found")
                .block(Block::bordered().title("Error"))
                .style(Style::new().red())
                .alignment(Alignment::Center);

            frame.render_widget(error_widget, frame.area())
        }
        Errors::None => {
            if state.loading {
                let loading_widget = Paragraph::new("Loading data, please wait...")
                    .block(Block::bordered().title("Status"))
                    .style(Style::new().yellow())
                    .alignment(Alignment::Center);

                frame.render_widget(loading_widget, frame.area())
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
                    .header(Row::new(vec!["Username", "# of Items"]));
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

                let layer_info_text = format!(
                    "Title: {selected_title}\nOwner: {selected_owner}\nReferences Last Synced: {last_sync}\n<j>/<Down> Navigate Down | <k>/<Up> Navigate Up\n<Enter> to refresh data | <f> to filter by username | <0> zero references"
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
                // let block = Block::bordered().title("Search Term");
                // frame.render_widget(Clear, layout[1]);
                // frame.render_widget(block, layout[1])
                // } else {
                frame.render_stateful_widget(widget_left, layout[0], &mut state.list_state);

                frame.render_widget(widget_center, layout[1]);

                frame.render_widget(widget_right, layout[2]);
                // }
            }
        }
    }
}
