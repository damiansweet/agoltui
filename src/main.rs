use clap::Parser;
use crossterm::event::{self, Event};
use std::sync::Arc;
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
    #[error("AGOL Data Fetch error: {0}")]
    FetchAll(#[from] std::boxed::Box<dyn std::error::Error + Send + Sync>),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = ui::Args::parse();
    let mut terminal = ratatui::init();
    //TODO create a loading screen widget to display data is fetching in background

    let client = Arc::new(reqwest::Client::new());
    let access_token = Arc::new(agol::fetch_oauth2_agol_token(&client).await?);

    let total_agol_count = agol::fetch_agol_content_total_count(&client, &access_token).await?;

    let results = agol::fetch_all_agol_content(
        Arc::clone(&client),
        Arc::clone(&access_token),
        total_agol_count,
    )
    .await?;

    let mut ui_state = ui::init_state(args, results);
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
