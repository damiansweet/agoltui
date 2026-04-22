use agol::models::{AgolItemType, ArcGISAccessToken, ArcGISReferences, ArcGISSearchResults};
use clap::Parser;
use crossterm::event::{self, Event};
use std::sync::Arc;
use thiserror::Error;

use std::collections::{HashMap, HashSet};

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
) -> Result<(Vec<ArcGISSearchResults>, ArcGISReferences), AppError> {
    let total_agol_count = agol::fetch_agol_content_total_count(&client, &access_token).await?;
    let results =
        agol::fetch_all_agol_content(client.clone(), access_token.clone(), total_agol_count)
            .await?;

    let mut references = ArcGISReferences {
        lookup: HashMap::new(),
    };

    let source_data: Vec<&ArcGISSearchResults> = results
        .iter()
        .filter(|i| {
            matches!(
                AgolItemType::try_from(i.item_type.as_str()),
                Ok(AgolItemType::SourceData(_))
            )
        })
        .collect();

    for source in source_data {
        references.lookup.insert(source.id.clone(), HashSet::new());
    }

    let web_apps: Vec<&ArcGISSearchResults> = results
        .iter()
        .filter(|i| {
            matches!(
                AgolItemType::try_from(i.item_type.as_str()),
                Ok(AgolItemType::WebApp(_))
            )
        })
        .collect();

    for web_app in web_apps {
        let item_type = AgolItemType::try_from(web_app.item_type.as_str()).unwrap();
        let tmp_references =
            agol::fetch_per_web_app_type(&client, &access_token, web_app, item_type).await?;
        for (k, v) in tmp_references.lookup {
            references.lookup.entry(k).or_default().extend(v);
        }
    }

    Ok((results, references))
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = ui::Args::parse();
    let mut terminal = ratatui::init();

    let client = Arc::new(reqwest::Client::new());
    let access_token = Arc::new(agol::fetch_oauth2_agol_token(&client).await?);

    let (agol_items, references) =
        fetch_agol_data(Arc::clone(&client), Arc::clone(&access_token)).await?;

    //TODO initialize item_references []
    //TODO initialize broken_connections []
    //TODO filter out source data []
    //TODO call agol::fetch_data_per_web_app []

    let mut ui_state = ui::init_state(args, agol_items, references);
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
