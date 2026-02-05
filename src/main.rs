use agol::models::ArcGISSearchResults;
use crossterm::event::{self, Event, KeyCode};
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListDirection, ListItem, ListState, Paragraph},
};
use std::path::Path;

//TODO display feature layer info that has the most references
//
//TODO on right widget show references for focused left widget agol item_id

#[derive(Debug, Clone)]
struct UiState {
    selected: Option<usize>,
    list_state: ListState,
}

fn init_state(len: usize) -> UiState {
    let mut list_state = ListState::default();
    let selected = if len == 0 { None } else { Some(0) };
    list_state.select(selected);
    UiState {
        selected,
        list_state,
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
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
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

    let widget_right = Paragraph::new(format!("Selected Title: {selected_title}"))
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_stateful_widget(widget_left, layout[0], &mut state.list_state);

    frame.render_widget(widget_right, layout[1]);
}

fn load_all_content_from_file() -> Result<Vec<ArcGISSearchResults>, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string("data/all_agol_content.json")?;

    let data = serde_json::from_str(&data)?;
    Ok(data)
}
fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    //TODO create a loading screen widget to display data is fetching in background

    let client = reqwest::blocking::Client::new();
    let access_token = agol::fetch_oath2_agol_token_blocking(&client);

    match access_token {
        Ok(_access_token) => {
            // let all_agol_content = agol::fetch_all_agol_content_blocking(&client, &access_token);
            let all_agol_content = load_all_content_from_file();
            //TODO create function that refreshes content in json file and re-reads from same file
            //TODO show on bottom line last time data was synced

            match all_agol_content {
                Ok(agol_content) => {
                    let relative_path = Path::new("data/all_agol_content.json");
                    agol::pretty_write_all_agol_content_to_file(relative_path, &agol_content)
                        .expect("unable to write all agol content to json");

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

                                KeyCode::Enter => {
                                    if let Some(item) = selected_item(&ui_state, &agol_content) {
                                        //todo use item id to fetch references and populate right widget
                                        println!("selected item id: {}", item.id);
                                    }
                                }

                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("failed to fetch all agol content: {e}");
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
