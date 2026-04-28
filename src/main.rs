use agol::models::{AgolItemType, ArcGISAccessToken, ArcGISReferences, ArcGISSearchResults};
use clap::Parser;
use crossterm::event::{self, Event};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use thiserror::Error;

use std::collections::HashMap;

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

async fn fetch_agol_data(
    client: Arc<reqwest::Client>,
    access_token: Arc<ArcGISAccessToken>,
    total_agol_count: u32,
) -> Result<Vec<ArcGISSearchResults>, AppError> {
    let results =
        agol::fetch_all_agol_content(client.clone(), access_token.clone(), total_agol_count)
            .await?;

    // let mut references = ArcGISReferences {
    //     lookup: HashMap::new(),
    // };

    Ok(results)
}

async fn process_references_only(
    client: Arc<reqwest::Client>,
    access_token: Arc<ArcGISAccessToken>,
    results: Vec<ArcGISSearchResults>,
) -> Result<ArcGISReferences, AppError> {
    let mut references = ArcGISReferences {
        lookup: HashMap::new(),
    };

    let mut stream_of_futures =
        stream::iter(results.clone())
            .map(|s| {
                let client = Arc::clone(&client);
                let access_token = Arc::clone(&access_token);
                let item_type = AgolItemType::try_from(s.item_type.as_str());
                async move {
                    agol::fetch_per_agol_item_type(&client, &access_token, &s, item_type).await
                }
            })
            .buffer_unordered(100);

    while let Some(web_app_references) = stream_of_futures.next().await {
        match web_app_references {
            Ok(r) => {
                for (k, v) in r.lookup {
                    references.lookup.entry(k).or_default().extend(v);
                }
            }
            Err(e) => panic!("arcgis lib error: {:#?}", e),
        }
    }

    Ok(references)
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = ui::Args::parse();
    let mut terminal = ratatui::init();

    let client = Arc::new(reqwest::Client::new());
    let access_token = Arc::new(agol::fetch_oauth2_agol_token(&client).await?);

    let total_agol_count = agol::fetch_agol_content_total_count(&client, &access_token).await?;
    let agol_items = fetch_agol_data(
        Arc::clone(&client),
        Arc::clone(&access_token),
        total_agol_count,
    )
    .await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ArcGISReferences>(1);

    let client_bg = Arc::clone(&client);
    let token_bg = Arc::clone(&access_token);
    let items_bg = agol_items.clone();

    tokio::spawn(async move {
        if let Ok(refs) = process_references_only(client_bg, token_bg, items_bg).await {
            let _ = tx.send(refs).await;
        }
    });

    let mut ui_state = ui::init_state(
        args,
        agol_items,
        total_agol_count,
        ArcGISReferences::default(),
    );
    ui_state.references_loading = true;

    while ui_state.running {
        terminal.draw(|frame| ui::ui(frame, &mut ui_state))?;

        if let Ok(refs) = rx.try_recv() {
            ui_state.references_lookup = refs;
            ui_state.references_loading = false;
        }

        if !event::poll(std::time::Duration::from_millis(16))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            let action = action::handle_key(&ui_state, key.code);
            action::handle_action(&mut ui_state, &mut terminal, action, &client, &access_token)
                .await;
        }
    }

    ratatui::restore();
    Ok(())
}
