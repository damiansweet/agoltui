use clap::Parser;
use crossterm::event::{self, Event};
use thiserror::Error;

mod action;
mod ui;
mod utils;

//TODO display feature layer info that has the most references

#[derive(Error, Debug)]
enum AppError {
    #[error("AGOL Lib error: {0}")]
    Agol(#[from] agol::error::ArcGISLibError),
    #[error("Ratatui error: {0}")]
    Ratatui(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = ui::Args::parse();
    let mut terminal = ratatui::init();
    //TODO create a loading screen widget to display data is fetching in background

    let client = reqwest::Client::new();
    let access_token = agol::fetch_oauth2_agol_token(&client).await?;

    // let all_agol_content = agol::fetch_all_agol_content_blocking(&client, &access_token);
    let _all_agol_content = utils::load_all_content_from_file();

    let mut ui_state = ui::init_state(args);
    while ui_state.running {
        terminal.draw(|frame| ui::ui(frame, &mut ui_state))?;

        if let Event::Key(key) = event::read()? {
            let action = action::handle_key(&ui_state, key.code);
            action::handle_action(&mut ui_state, &mut terminal, action, &client, &access_token)
                .await;
        }
    }

    ratatui::restore();
    Ok(())
}
