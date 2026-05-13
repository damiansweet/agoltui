use crate::errors::AppError;

use agol::models::{ArcGISReferences, ArcGISSearchResults};
use clap::Parser;
use crossterm::event::{self, Event};
use std::sync::Arc;

use std::collections::HashSet;

use crate::ui::{Agol, Config};

mod action;
mod agol_data;
mod errors;
mod models;
mod ui;
mod utils;

//TODO display feature layer info that has the most references

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = ui::Args::parse();
    let mut terminal = ratatui::init();

    let client = Arc::new(reqwest::Client::new());
    let access_token = Arc::new(agol::fetch_oauth2_agol_token(&client).await?);

    let config = Config {
        org_info: agol::fetch_org_info(&client, &access_token).await?,
        access_token: access_token.clone(),
    };

    let total_agol_count =
        agol::fetch_agol_content_total_count(&client, &access_token, &config.org_info.org_id)
            .await?;
    let agol_items = agol_data::fetch_agol_data(
        Arc::clone(&client),
        Arc::clone(&config.access_token),
        total_agol_count,
        &config.org_info.org_id,
    )
    .await?;

    let agol = Agol {
        agol_content: agol_items.clone(),
        cached_agol_content: agol_items,
        references: ArcGISReferences::default(),
    };

    let valid_agol_item_ids = agol::extract_item_ids(&agol.agol_content);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ArcGISReferences>(1);

    let client_bg = Arc::clone(&client);
    let token_bg = Arc::clone(&config.access_token);
    let items_bg = agol.agol_content.clone();

    tokio::spawn(async move {
        if let Ok(refs) = agol_data::process_references_only(client_bg, token_bg, items_bg).await {
            let _ = tx.send(refs).await;
        }
    });

    let mut app = ui::init_state(args, agol.clone(), config.clone());
    app.state.references_loading = true;

    while app.state.running {
        terminal.draw(|frame| ui::ui(frame, &mut app))?;

        if let Ok(mut refs) = rx.try_recv() {
            let mut broken_connections: HashSet<ArcGISSearchResults> = HashSet::new();

            for (k, v) in &refs.lookup {
                if !valid_agol_item_ids.contains(&k.as_str()) {
                    for j in v {
                        broken_connections.insert(j.clone());
                    }
                }
            }
            refs.broken_connections = broken_connections;

            app.agol.references = refs;
            app.state.references_loading = false;
        }

        if !event::poll(std::time::Duration::from_millis(16))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            let action = action::handle_key(&app.state, key);
            action::handle_action(&mut app, action).await;
        }
    }
    //TODO match on specific error and render match error widget

    ratatui::restore();
    Ok(())
}
