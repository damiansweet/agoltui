use std::collections::HashMap;

use agol::models::{ArcGISReferences, ArcGISSearchResults};
use clap::Parser;

use crate::utils;
use ratatui::style::{Color, Style};
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
    pub agol_total_count: u32,
    pub references_lookup: ArcGISReferences,
    pub selected: Option<usize>,
    pub list_state: ListState,
    pub running: bool,
    pub loading: bool,
    pub references_loading: bool,
    pub search_popup: bool,
    pub user_input: UserInput,
    pub search_type: SearchType,
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

#[derive(Debug, Default, PartialEq)]
pub enum SearchType {
    #[default]
    Title,
    Owner,
}

#[derive(Debug)]
pub enum Errors {
    NoAccessToken,
    EmailNotFound,
    NoExistingData,
    InvalidUserInput,
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
pub fn init_state(
    cli_input: Args,
    agol_content: Vec<agol::models::ArcGISSearchResults>,
    agol_total_count: u32,
    references_lookup: ArcGISReferences,
) -> UiState {
    let mut list_state = ListState::default();
    let selected = Some(0);
    list_state.select(selected);
    let running = true;
    let references_loading = false;
    let loading = false;
    let search_popup = false;
    let input_mode = InputMode::Normal;

    let input = match &cli_input.email {
        Some(email) => email.to_string(),
        None => String::default(),
    };

    //TODO put in logic to set user_input dynamically based on clap arguments passed in
    //TODO validate user input is between 2-30 chars
    let user_input = UserInput {
        input,
        character_index: 0,
    };

    let usernames = HashMap::new();

    let mut username_state = TableState::default();
    username_state.select(Some(0));

    let mut errors = None;

    let search_type = SearchType::default();
    let queries = Vec::new();

    // let agol_content = if let Ok(agol_content) = utils::load_all_content_from_file() {
    //     agol_content
    // } else {
    //     errors = Some(Errors::NoExistingData);
    //     Vec::new()
    //     // let _ = utils::refresh_data(&client, &access_token);
    //     // utils::load_all_content_from_file().expect("unable to read from all content json")
    // };

    UiState {
        agol_content,
        agol_total_count,
        selected,
        list_state,
        running,
        loading,
        references_loading,
        search_popup,
        user_input,
        input_mode,
        search_type,
        cli_input,
        usernames,
        username_state,
        errors,
        queries,
        references_lookup,
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

fn invalid_user_input_widget() -> Paragraph<'static> {
    Paragraph::new("Query must be between 3-50 characters")
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

fn app_launch_widget() -> Paragraph<'static> {
    Paragraph::new("Welcome to AgolTUI :)\n Please press Enter to fetch AGOL content")
        .block(Block::bordered())
        .style(Style::new().green())
        .alignment(Alignment::Center)
}

//TODO create widget that shows filter/search combos at bottom of screen

pub fn ui(frame: &mut Frame, state: &mut UiState) {
    //TODO figure out way to have app_launch_widget be opening default
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
        Some(Errors::InvalidUserInput) => {
            frame.render_widget(invalid_user_input_widget(), frame.area());
        }
        None => {
            if state.search_popup {
                let query = state.user_input.input.clone();
                let user_input = match state.search_type {
                    SearchType::Title => Paragraph::new(query)
                        .style(Style::new().light_blue())
                        .block(Block::bordered().title("Enter Search Term")),
                    SearchType::Owner => Paragraph::new(query)
                        .style(Style::new().yellow())
                        .block(Block::bordered().title("Enter Username")),
                };

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
                } else if !state.usernames.is_empty() {
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

                    let num_list_items = state.agol_content.len();

                    let widget_top = List::new(all_content_ids)
                        .block(
                            Block::bordered()
                                .title_alignment(Alignment::Center)
                                .title(format!("All AGOL Content List\t {num_list_items}")),
                        )
                        .style(Style::new().white())
                        .highlight_style(Style::new().italic())
                        .highlight_symbol(">>")
                        .repeat_highlight_symbol(true)
                        .direction(ListDirection::TopToBottom);

                    // let content_count = all_agol_content.len();
                    let selected_title = selected_item(state, &state.agol_content)
                        .map(|item| item.title.as_str())
                        .unwrap_or_default();

                    let selected_item_type = selected_item(state, &state.agol_content)
                        .map(|item| item.item_type.as_str())
                        .unwrap_or_default();

                    let selected_owner = selected_item(state, &state.agol_content)
                        .map(|item| item.owner.as_str())
                        .unwrap_or_default();

                    let queries = &state.queries.join(" && ");

                    let layer_info_text = format!(
                        "Title: {selected_title}\nItem Type: {selected_item_type}\nOwner: {selected_owner}\n<j>/<Down> Navigate Down | <k>/<Up> Navigate Up\n<f> filter by username | <0> zero references\nCurrent Query: {queries}"
                    );

                    let widget_center = if state.references_loading {
                        Paragraph::new("Loading references...")
                            .block(Block::bordered().title("References"))
                            .style(Style::new().yellow())
                    } else {
                        Paragraph::new(layer_info_text)
                            .wrap(Wrap { trim: true })
                            .block(Block::bordered().title("Layer Info"))
                            .style(Style::new().white())
                            .alignment(Alignment::Center)
                    };

                    let widget_bottom = if let Some(selected_id) =
                        selected_item(state, &state.agol_content).map(|item| item.id.as_str())
                    {
                        //TODO style selected table item and add to UiState
                        let references = utils::get_layer_references(selected_id, state);
                        let mut sorted_references: Vec<ArcGISSearchResults> = Vec::new();
                        for r in &references {
                            sorted_references.push(r.clone());
                        }
                        sorted_references.sort_by(|a, b| a.title.cmp(&b.title));
                        // references.sort_by(|a, b| a);
                        let header = Row::new(["Title", "Type", "Url"])
                            .style(Style::new().bold())
                            .bottom_margin(1);

                        let mut rows: Vec<Row> = Vec::new();
                        for r in sorted_references {
                            let url = format!(
                                "https://cityoflonetree.maps.arcgis.com/home/item.html?id={}",
                                &r.id
                            );
                            rows.push(Row::new([r.title, r.item_type, url]));
                        }
                        //TODO change this to table

                        let widths = [
                            Constraint::Percentage(30),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .column_spacing(1)
                            .style(Color::White)
                    } else {
                        let header = Row::new(["Title", "Type", "Url"]);
                        let rows: Vec<Row> = Vec::new();
                        let widths = [
                            Constraint::Percentage(30),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .column_spacing(1)
                            .style(Color::White)
                    };

                    // if state.search_popup {
                    // } else {
                    frame.render_stateful_widget(widget_top, layout[0], &mut state.list_state);

                    frame.render_widget(widget_center, layout[1]);

                    frame.render_widget(widget_bottom, layout[2]);
                    // }
                }
            }
        }
    }
}
