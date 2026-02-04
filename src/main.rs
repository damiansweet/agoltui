use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};

//TODO display feature layer info that has the most references

fn ui(frame: &mut Frame) {
    // let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.area());

    let widget_top = Paragraph::new("Hello There Damian!")
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    let widget_bottom = Paragraph::new("Hello There AGAIN Damian!")
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(widget_top, layout[0]);

    frame.render_widget(widget_bottom, layout[1]);
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let client = reqwest::blocking::Client::new();
    let access_token = agol::fetch_oath2_agol_token_blocking(&client);

    match access_token {
        Ok(access_token) => {
            let all_agol_content = agol::fetch_all_agol_content_blocking(&client, &access_token);

            match all_agol_content {
                Ok(agol_content) => {
                    let mut app_running = true;

                    while app_running {
                        terminal.draw(ui)?;

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
