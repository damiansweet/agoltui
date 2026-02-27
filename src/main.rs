use clap::Parser;
use crossterm::event::{self, Event};

mod action;
mod ui;
mod utils;

//TODO display feature layer info that has the most references

fn main() -> std::io::Result<()> {
    let args = ui::Args::parse();
    let mut terminal = ratatui::init();
    //TODO create a loading screen widget to display data is fetching in background

    let client = reqwest::blocking::Client::new();
    let access_token = agol::fetch_oath2_agol_token_blocking(&client);

    match access_token {
        Ok(access_token) => {
            // let all_agol_content = agol::fetch_all_agol_content_blocking(&client, &access_token);
            let _all_agol_content = utils::load_all_content_from_file();

            let mut ui_state = ui::init_state(args);
            while ui_state.running {
                terminal.draw(|frame| ui::ui(frame, &mut ui_state))?;

                if let Event::Key(key) = event::read()? {
                    let action = action::handle_key(&ui_state, key.code);
                    action::handle_action(
                        &mut ui_state,
                        &mut terminal,
                        action,
                        &client,
                        &access_token,
                    );
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
