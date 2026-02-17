use agol::filter_feature_services;
use agol::models::ArcGISSearchResults;
use chrono::Local;
use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Clear, List, ListDirection, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

mod action;

//TODO display feature layer info that has the most references

#[derive(Debug)]
pub struct UiState {
    agol_content: Vec<ArcGISSearchResults>,
    selected: Option<usize>,
    list_state: ListState,
    last_synced: String,
    running: bool,
    loading: bool,
    search_popup: bool,
    username_popup: bool,
    user_input: UserInput,
    cli_input: Args,
    usernames: Vec<String>,
}

#[derive(Debug)]
pub struct UserInput {
    input: String,
    character_index: usize,
    input_mode: InputMode,
}

#[derive(Debug)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Email of the user to search
    #[arg(short, long)]
    email: Option<String>,

    /// Search term to filter results
    #[arg(short, long)]
    search: Option<String>,
}

fn format_email(cli_input: &mut Args) {
    match cli_input.email.as_deref() {
        Some(email) if email.eq_ignore_ascii_case("damian.sweet@cityoflonetree.com") => {
            cli_input.email = Some("Damian.Sweet@cityoflonetree.com".to_string())
        }
        Some(email) if email.eq_ignore_ascii_case("courtland.langley@cityoflonetree.com") => {
            cli_input.email = Some("courtland.langley@cityoflonetree.com".to_string())
        }
        _ => {}
    }
}

fn init_state(
    len: usize,
    client: &reqwest::blocking::Client,
    access_token: &agol::models::ArcGISAccessToken,
    cli_input: Args,
) -> UiState {
    let mut list_state = ListState::default();
    let selected = if len == 0 { None } else { Some(0) };
    list_state.select(selected);
    let last_synced = read_last_sync();
    let running = true;
    let loading = false;
    let search_popup = false;
    let username_popup = false;
    let user_input = UserInput {
        input: String::new(),
        input_mode: InputMode::Normal,
        character_index: 0,
    };
    let usernames = Vec::new();

    let mut cli_input = cli_input;
    format_email(&mut cli_input);

    let agol_content = if let Ok(agol_content) = load_all_content_from_file() {
        agol_content
    } else {
        let _ = refresh_data(&client, &access_token);
        load_all_content_from_file().expect("unable to read from all content json")
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
    }
}

pub fn read_last_sync() -> String {
    let file = std::fs::read_to_string("data/last_sync.txt");

    match file {
        Ok(file_contents) => {
            if file_contents.len() > 0 {
                // let sync_time: Vec<&str> = file.split("\n").collect();

                // let sync_time = sync_time[0].to_string();
                let sync_time = file_contents.trim();
                sync_time.to_string()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

//TODO test how many results come from this
pub fn filter_layer_no_references() -> Vec<ArcGISSearchResults> {
    if let Ok(file) = std::fs::read_to_string("data/all_layers_with_web_maps.json") {
        let layers_with_references: HashMap<String, Vec<String>> =
            serde_json::from_str(&file).expect("unable to convert json file to HashMap");
        let layer_with_references: Vec<String> = layers_with_references
            .into_iter()
            .filter(|(_layer, references)| !references.is_empty())
            .map(|(layer, _references)| layer)
            .collect();

        if let Ok(file) = std::fs::read_to_string("data/all_agol_content.json") {
            let all_content: Vec<ArcGISSearchResults> =
                serde_json::from_str(&file).expect("unable to convert json to struct");
            all_content
                .into_iter()
                .filter(|content| {
                    !layer_with_references.contains(&content.id)
                        && content.item_type == "Feature Service"
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

//TODO call this from action not UI
fn get_layer_references(id: &str) -> Vec<String> {
    let file = std::fs::read_to_string("data/all_layers_with_web_maps.json");

    let file_string = match file {
        Ok(file_string) => file_string,
        Err(_) => String::from(""),
    };

    let layer_references: HashMap<String, Vec<String>> =
        serde_json::from_str(&file_string).unwrap_or_default();

    if let Some(references) = layer_references.get(id) {
        // references.clone()
        references
            .into_iter()
            .map(|item| format!("https://cityoflonetree.maps.arcgis.com/home/item.html?id={item}"))
            .collect()
    } else {
        Vec::new()
    }
}

fn selected_item<'a>(
    state: &UiState,
    items: &'a [ArcGISSearchResults],
) -> Option<&'a ArcGISSearchResults> {
    state.selected.and_then(|i| items.get(i))
}

fn ui(frame: &mut Frame, state: &mut UiState) {
    if state.loading {
        let loading_widget = Paragraph::new("Loading data, please wait...")
            .block(Block::bordered().title("Status"))
            .style(Style::new().yellow())
            .alignment(Alignment::Center);

        frame.render_widget(loading_widget, frame.area())
    } else if state.usernames.len() >= 1 {
        let all_usernames: Vec<ListItem> = state
            .usernames
            .iter()
            .map(|item| ListItem::new(item.as_str()))
            .collect();

        let widget_left = List::new(all_usernames)
            .block(Block::bordered().title("All AGOL Content List"))
            .style(Style::new().white())
            .highlight_style(Style::new().italic())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);
        frame.render_widget(widget_left, frame.area());
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
            let references = get_layer_references(selected_id);
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

        if state.search_popup {
            let block = Block::bordered().title("Search Term");
            frame.render_widget(Clear, layout[1]);
            frame.render_widget(block, layout[1])
        } else {
            frame.render_stateful_widget(widget_left, layout[0], &mut state.list_state);

            frame.render_widget(widget_center, layout[1]);

            frame.render_widget(widget_right, layout[2]);
        }
    }
}

fn load_all_content_from_file() -> Result<Vec<ArcGISSearchResults>, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string("data/all_agol_content.json")?;

    let data = serde_json::from_str(&data)?;
    Ok(data)
}

pub fn refresh_data(
    client: &reqwest::blocking::Client,
    access_token: &agol::models::ArcGISAccessToken,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let all_agol_content = agol::fetch_all_agol_content_blocking(client, access_token)?;
    let all_agol_content_path = Path::new("data/all_agol_content.json");
    agol::pretty_write_all_agol_content_to_file(&all_agol_content_path, &all_agol_content)?;

    let web_maps = agol::filter_web_maps(&all_agol_content);
    let web_map_ids = agol::extract_agol_ids(&web_maps);

    let all_layers_with_web_maps =
        agol::fetch_layers_for_all_web_maps_blocking(client, access_token, &web_map_ids)?;
    //write all layers with web maps to json file

    let layers_with_web_map_path = Path::new("data/all_layers_with_web_maps.json");
    pretty_write_all_layers_with_web_maps_to_file(
        layers_with_web_map_path,
        all_layers_with_web_maps,
    )?;

    let current_time = get_current_time()?;

    fs::write("data/last_sync.txt", current_time)?;

    Ok(())
}

fn pretty_write_all_layers_with_web_maps_to_file(
    file_path: &Path,
    all_layers: HashMap<String, HashSet<String>>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(file_path).expect("failed to create file");
    let writer = std::io::BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &all_layers)?;

    Ok(())
}

fn get_current_time() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let dt = Local::now();
    let date_string = dt.to_rfc2822();
    Ok(date_string)
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let mut terminal = ratatui::init();
    //TODO create a loading screen widget to display data is fetching in background

    let client = reqwest::blocking::Client::new();
    let access_token = agol::fetch_oath2_agol_token_blocking(&client);

    match access_token {
        Ok(access_token) => {
            // let all_agol_content = agol::fetch_all_agol_content_blocking(&client, &access_token);
            let all_agol_content = load_all_content_from_file();
            //TODO create function that refreshes content in json file and re-reads from same file
            //TODO show on bottom line last time data was synced

            match all_agol_content {
                Ok(agol_content) => {
                    let agol_content = filter_feature_services(&agol_content);
                    let mut ui_state = init_state(agol_content.len(), &client, &access_token, args);

                    while ui_state.running {
                        terminal.draw(|frame| ui(frame, &mut ui_state))?;

                        if let Event::Key(key) = event::read()? {
                            let action = action::handle_key(key.code);
                            action::handle_action(
                                &mut ui_state,
                                &mut terminal,
                                action,
                                &client,
                                &access_token,
                            );
                        }
                    }
                }

                Err(_) => match refresh_data(&client, &access_token) {
                    Ok(_) => {}

                    Err(e) => {
                        eprintln!("error refreshing data: {:?}", e);
                    }
                },
            }

            ratatui::restore();
        }
        Err(e) => {
            eprintln!("failed to fetch access_token: {}", e);
        }
    }
    Ok(())
}
