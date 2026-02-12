use agol::filter_feature_services;
use agol::models::ArcGISSearchResults;
use chrono::Local;
use crossterm::event::{self, Event};
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, List, ListDirection, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

mod action;

//TODO display feature layer info that has the most references

#[derive(Debug, Clone)]
pub struct UiState {
    selected: Option<usize>,
    list_state: ListState,
    last_synced: String,
    running: bool,
    loading: bool,
}

fn init_state(len: usize) -> UiState {
    let mut list_state = ListState::default();
    let selected = if len == 0 { None } else { Some(0) };
    list_state.select(selected);
    let last_synced = read_last_sync();
    let running = true;
    let loading = false;

    UiState {
        selected,
        list_state,
        last_synced,
        running,
        loading,
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

fn ui(
    frame: &mut Frame,
    all_agol_content: &[agol::models::ArcGISSearchResults],
    state: &mut UiState,
) {
    if state.loading {
        let loading_widget = Paragraph::new("Loading data, please wait...")
            .block(Block::bordered().title("Status"))
            .style(Style::new().yellow())
            .alignment(Alignment::Center);

        frame.render_widget(loading_widget, frame.area())
    } else {
        // let area = frame.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(10),
                Constraint::Percentage(40),
            ])
            .split(frame.area());

        let all_content_ids: Vec<ListItem> = all_agol_content
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
        let selected_title = selected_item(state, all_agol_content)
            .map(|item| item.title.as_str())
            .unwrap_or_default();

        let selected_owner = selected_item(state, all_agol_content)
            .map(|item| item.owner.as_str())
            .unwrap_or_default();
        let last_sync = &state.last_synced.clone();

        let layer_info_text = format!(
            "Title: {selected_title}\nOwner: {selected_owner}\nReferences Last Synced: {last_sync}"
        );

        let widget_center = Paragraph::new(layer_info_text)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title("Layer Info"))
            .style(Style::new().white())
            .alignment(Alignment::Center);

        let widget_right = if let Some(selected_id) =
            selected_item(state, all_agol_content).map(|item| item.id.as_str())
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

        frame.render_stateful_widget(widget_left, layout[0], &mut state.list_state);

        frame.render_widget(widget_center, layout[1]);

        frame.render_widget(widget_right, layout[2]);
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

#[allow(dead_code)]
fn get_current_time() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let dt = Local::now();
    let date_string = dt.to_rfc2822();
    Ok(date_string)
}

fn main() -> std::io::Result<()> {
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
                    let mut ui_state = init_state(agol_content.len());

                    while ui_state.running {
                        terminal.draw(|frame| ui(frame, &agol_content, &mut ui_state))?;

                        if let Event::Key(key) = event::read()? {
                            let action = action::handle_key(key.code);
                            action::handle_action(
                                &mut ui_state,
                                &mut terminal,
                                agol_content.len(),
                                agol_content.clone(),
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
