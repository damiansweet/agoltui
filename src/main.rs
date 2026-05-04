use agol::models::{AgolItemType, ArcGISAccessToken, ArcGISReferences, ArcGISSearchResults};
use clap::Parser;
use crossterm::event::{self, Event};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use thiserror::Error;

use std::collections::{HashMap, HashSet};

use crate::ui::{Agol, Config};

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
    org_id: &str,
) -> Result<Vec<ArcGISSearchResults>, AppError> {
    let results = agol::fetch_all_agol_content(
        client.clone(),
        access_token.clone(),
        total_agol_count,
        org_id,
    )
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
        broken_connections: HashSet::new(),
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

    let config: Config = Config {
        org_info: agol::fetch_org_info(&client, &access_token).await?,
        access_token: access_token.clone(),
    };

    let total_agol_count =
        agol::fetch_agol_content_total_count(&client, &access_token, &config.org_info.org_id)
            .await?;
    let agol_items = fetch_agol_data(
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
        if let Ok(refs) = process_references_only(client_bg, token_bg, items_bg).await {
            let _ = tx.send(refs).await;
        }
    });

    let mut ui_state = ui::init_state(args, agol.clone(), config.clone());
    ui_state.references_loading = true;

    while ui_state.running {
        terminal.draw(|frame| ui::ui(frame, &mut ui_state))?;

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

            ui_state.agol.references = refs;
            ui_state.references_loading = false;
        }

        if !event::poll(std::time::Duration::from_millis(16))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            let action = action::handle_key(&ui_state, key);
            action::handle_action(&mut ui_state, action).await;
        }
    }
    //TODO match on specific error and render match error widget

    ratatui::restore();
    Ok(())
}
