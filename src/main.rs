use agol::filter_feature_services;
use agol::models::ArcGISSearchResults;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode};
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, List, ListDirection, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

//TODO display feature layer info that has the most references
//
//TODO on right widget show references for focused left widget agol item_id

#[derive(Debug, Clone)]
struct UiState {
    selected: Option<usize>,
    list_state: ListState,
    last_synced: Option<String>,
}

fn init_state(len: usize) -> UiState {
    let mut list_state = ListState::default();
    let selected = if len == 0 { None } else { Some(0) };
    list_state.select(selected);
    let last_synced = read_last_sync();

    let last_synced = match last_synced {
        Ok(Some(time)) => Some(time),
        Ok(None) => None,
        _ => None,
    };
    UiState {
        selected,
        list_state,
        last_synced,
    }
}

fn read_last_sync() -> std::result::Result<Option<String>, Box<dyn std::error::Error>> {
    let file = std::fs::read_to_string("data/last_sync.txt")?;

    if file.len() > 0 {
        // let sync_time: Vec<&str> = file.split("\n").collect();

        // let sync_time = sync_time[0].to_string();
        let sync_time = file.trim();
        Ok(Some(sync_time.to_string()))
    } else {
        Ok(None)
    }
}

fn get_layer_references(id: &str) -> Vec<String> {
    let file = std::fs::read_to_string("data/all_layers_with_web_maps.json");

    let file_string = match file {
        Ok(file_string) => file_string,
        Err(_) => String::from(""),
    };

    let layer_references: HashMap<String, Vec<String>> =
        serde_json::from_str(&file_string).unwrap_or_default();

    if let Some(references) = layer_references.get(id) {
        references.clone()
    } else {
        Vec::new()
    }
}

fn move_selection(current: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let cur = current.unwrap_or(0) as isize;
    let len_i = len as isize;
    Some(((cur + delta).rem_euclid(len_i)) as usize)
}

fn apply_key(mut state: UiState, len: usize, code: KeyCode) -> UiState {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            let next = move_selection(state.selected, len, 1);
            state.selected = next;
            state.list_state.select(next);
            state
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let previous = move_selection(state.selected, len, -1);
            state.selected = previous;
            state.list_state.select(previous);
            state
        }
        _ => state,
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
    // let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
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
        .unwrap_or("<none>");

    let widget_center = Paragraph::new(selected_title)
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
                .style(Style::new().red())
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

fn load_all_content_from_file() -> Result<Vec<ArcGISSearchResults>, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string("data/all_agol_content.json")?;

    let data = serde_json::from_str(&data)?;
    Ok(data)
}

fn refresh_data(
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
                    let mut app_running = true;

                    while app_running {
                        terminal.draw(|frame| ui(frame, &agol_content, &mut ui_state))?;

                        if let Event::Key(key) = event::read()? {
                            match key.code {
                                KeyCode::Char('q') => app_running = false,

                                KeyCode::Char('j') | KeyCode::Down => {
                                    ui_state = apply_key(ui_state, agol_content.len(), key.code)
                                }

                                KeyCode::Char('k') | KeyCode::Up => {
                                    ui_state = apply_key(ui_state, agol_content.len(), key.code)
                                }

                                // KeyCode::Enter => {
                                //     if let Some(item) = selected_item(&ui_state, &agol_content) {
                                //         //todo use item id to fetch references and populate right widget
                                //         println!("selected item id: {}", item.id);
                                //         println!("last sync time: {:?}", ui_state.last_synced);
                                //     }
                                // }
                                _ => {}
                            }
                        }
                    }
                }
                Err(_) => {
                    match refresh_data(&client, &access_token) {
                        Ok(_) => {
                            let agol_content_refresh = load_all_content_from_file().unwrap();
                            let mut ui_state = init_state(agol_content_refresh.len());
                            let mut app_running = true;

                            while app_running {
                                terminal.draw(|frame| {
                                    ui(frame, &agol_content_refresh, &mut ui_state)
                                })?;

                                if let Event::Key(key) = event::read()? {
                                    match key.code {
                                        KeyCode::Char('q') => app_running = false,

                                        KeyCode::Char('j') | KeyCode::Down => {
                                            ui_state = apply_key(
                                                ui_state,
                                                agol_content_refresh.len(),
                                                key.code,
                                            )
                                        }

                                        KeyCode::Char('k') | KeyCode::Up => {
                                            ui_state = apply_key(
                                                ui_state,
                                                agol_content_refresh.len(),
                                                key.code,
                                            )
                                        }

                                        KeyCode::Enter => {
                                            if let Some(item) =
                                                selected_item(&ui_state, &agol_content_refresh)
                                            {
                                                //todo use item id to fetch references and populate right widget
                                                println!("selected item id: {}", item.id);
                                                // println!(
                                                // "last sync time: {:?}",
                                                // ui_state.last_synced
                                                // );
                                            }
                                        }

                                        _ => {}
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("error refreshing data: {:?}", e);
                        }
                    }
                }
            }

            ratatui::restore();
        }
        Err(e) => {
            eprintln!("failed to fetch access_token: {}", e);
        }
    }
    Ok(())
}
