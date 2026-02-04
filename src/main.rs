use agol::models::ArcGISSearchResults;
use crossterm::event::{self, Event, KeyCode};
use ratatui::style::Style;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListDirection, Paragraph},
};
use std::path::Path;

//TODO display feature layer info that has the most references
//
//TODO on right widget show references for focused left widget agol item_id

fn ui(frame: &mut Frame, all_agol_content: &[agol::models::ArcGISSearchResults]) {
    // let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.area());

    let all_content_ids: Vec<&str> = all_agol_content
        .iter()
        .map(|item| item.id.as_str())
        .collect();

    let widget_left = List::new(all_content_ids)
        .block(Block::bordered().title("All AGOL Content List"))
        .style(Style::new().white())
        .highlight_style(Style::new().italic())
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom);

    let content_count = all_agol_content.len();
    let widget_right = Paragraph::new(format!("total agol content: {content_count}"))
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(widget_left, layout[0]);

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

            match all_agol_content {
                Ok(agol_content) => {
                    let relative_path = Path::new("data/all_agol_content.json");
                    agol::pretty_write_all_agol_content_to_file(relative_path, &agol_content)
                        .expect("unable to write all agol content to json");

                    let mut app_running = true;

                    while app_running {
                        terminal.draw(|frame| ui(frame, &agol_content))?;

                        if let Event::Key(key) = event::read()? {
                            if let KeyCode::Char('q') = key.code {
                                app_running = false;
                            } else if let KeyCode::Char('a') = key.code {
                                println!("total agol content: {}", agol_content.len());
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
