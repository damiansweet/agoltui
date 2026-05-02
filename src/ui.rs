use std::collections::HashMap;

use agol::models::{ArcGISOrgInfo, ArcGISReferences, ArcGISSearchResults};
use clap::Parser;

use crate::utils;
use ratatui::style::{Color, Style};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position},
    widgets::{
        Block, Cell, Clear, List, ListDirection, ListItem, ListState, Paragraph, Row, Table,
        TableState, Wrap,
    },
};

#[derive(Debug)]
pub struct UiState {
    pub org_info: ArcGISOrgInfo,
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
    Id,
}

#[derive(Debug)]
pub enum Errors {
    NoAccessToken,
    // TODO fetch all org usernames
    //TODO create third widget and display if email not in org usernames
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
    org_info: ArcGISOrgInfo,
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

    let user_input = UserInput {
        input,
        character_index: 0,
    };

    let usernames = HashMap::new();

    let mut username_state = TableState::default();
    username_state.select(Some(0));

    let errors = None;

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
        org_info,
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

fn no_access_token_error_widget() -> Paragraph<'static> {
    Paragraph::new("No Access Token Found")
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

pub fn ui(frame: &mut Frame, state: &mut UiState) {
    match state.errors {
        Some(Errors::NoAccessToken) => {
            let no_access_token_error_widget = no_access_token_error_widget();

            frame.render_widget(no_access_token_error_widget, frame.area())
        }
        Some(Errors::InvalidUserInput) => {
            frame.render_widget(invalid_user_input_widget(), frame.area());
        }
        None => {
            if state.search_popup {
                let query = state.user_input.input.clone();
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(frame.area());
                let user_input = match state.search_type {
                    SearchType::Title => Paragraph::new(query)
                        .style(Style::new().light_blue())
                        .block(Block::bordered().title("Search by Keyword")),
                    SearchType::Owner => Paragraph::new(query)
                        .style(Style::new().yellow())
                        .block(Block::bordered().title("Search by Email")),
                    SearchType::Id => Paragraph::new(query)
                        .style(Style::new().light_cyan())
                        .block(Block::bordered().title("Search by Item Id")),
                };

                let key_binds =
                    "Search by Keyword: <F1>\nSearch by Email: <F2>\nSearch by Item Id: <F3>";

                let key_binds_widget = Paragraph::new(key_binds)
                    .style(Style::new().light_blue())
                    .block(Block::bordered().title("KeyBinds"));

                let input_area = frame.area();
                frame.render_widget(Clear, frame.area());
                frame.render_widget(user_input, layout[0]);
                frame.render_widget(key_binds_widget, layout[1]);

                //TODO possible remove match and always show cursor. maybe blink when editing
                match state.input_mode {
                    InputMode::Normal => {
                        frame.set_cursor_position(Position::new(
                            input_area.x + state.user_input.character_index as u16 + 1,
                            input_area.y + 1,
                        ));
                    }
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
                    let current_query = state.queries.clone();
                    let rows: Vec<Row> = state
                        .usernames
                        .iter()
                        .map(|(k, v)| Row::new(vec![k.to_string(), v.to_string()]))
                        .collect();

                    let widths = [Constraint::Length(80), Constraint::Length(20)];
                    let username_widget = Table::new(rows, widths)
                        .column_spacing(1)
                        .style(Style::new().blue())
                        .highlight_symbol(">>")
                        .header(Row::new(vec!["Username", "# of Items"]))
                        .footer(Row::new(vec![Cell::new(current_query.join(" && "))]))
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
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("Layer Info"),
                            )
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
                        let header = Row::new(["Index", "Title", "Type", "Url"])
                            .style(Style::new().bold())
                            .bottom_margin(1);

                        let mut rows: Vec<Row> = Vec::new();
                        for (i, r) in sorted_references.into_iter().enumerate() {
                            let url =
                                format!("{}/home/item.html?id={}", &state.org_info.full_url, &r.id);
                            rows.push(Row::new([i.to_string(), r.title, r.item_type, url]));
                        }
                        //TODO conditionally render no references if !sorted_references.is_empty()

                        let widths = [
                            Constraint::Percentage(5),
                            Constraint::Percentage(25),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .column_spacing(1)
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("References"),
                            )
                            .style(Color::White)
                    } else {
                        let header = Row::new(["Index", "Title", "Type", "Url"]);
                        let rows: Vec<Row> = Vec::new();
                        let widths = [
                            Constraint::Percentage(5),
                            Constraint::Percentage(25),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("No References"),
                            )
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
