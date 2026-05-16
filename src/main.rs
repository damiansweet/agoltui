use crate::{errors::AppError, models::CliArgsFilter};

use agol::models::{ArcGISReferences, ArcGISSearchResults, Users};
use clap::Parser;
use crossterm::event::{self, Event};
use std::sync::Arc;

use std::collections::HashSet;

use crate::models::{Agol, Args, Config};

mod action;
mod agol_data;
mod errors;
mod helix_keybinds;
mod models;
mod ui;
mod utils;
mod widgets;

//TODO display feature layer info that has the most references

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // let args = models::Args::parse();
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

    //TODO check cli args and filter agol_content accordingly

    let (cli_args_tx, mut cli_args_rx) = tokio::sync::mpsc::channel::<String>(1);

    let cli_args = Args::parse();
    let cli_filter = match cli_args {
        Args {
            email: Some(_),
            search: Some(_),
        } => CliArgsFilter::Both,

        Args {
            email: Some(_),
            search: None,
        } => CliArgsFilter::Email,
        Args {
            email: None,
            search: Some(_),
        } => CliArgsFilter::SearchTerm,
        Args {
            email: None,
            search: None,
        } => CliArgsFilter::None,
    };

    let agol_content = utils::filter_cli_args(&agol_items, &cli_args, cli_filter.clone());

    let cli_args_bg = cli_args.clone();
    let cli_filter_bg = cli_filter;

    tokio::spawn(async move {
        let query = utils::build_cli_args_query(cli_args_bg, cli_filter_bg).await;
        let _ = cli_args_tx.send(query).await;
    });

    let agol = Agol {
        agol_content,
        cached_agol_content: agol_items.iter().collect(),
        references: ArcGISReferences::default(),
        users: vec![Users::default()],
    };

    let valid_agol_item_ids = agol::extract_item_ids(&agol.agol_content);

    let (users_tx, mut users_rx) = tokio::sync::mpsc::channel::<Vec<Users>>(1);
    let (references_tx, mut references_rx) = tokio::sync::mpsc::channel::<ArcGISReferences>(1);

    let client_bg = Arc::clone(&client);
    let token_bg = Arc::clone(&config.access_token);
    let items_bg = agol_items.clone();

    tokio::spawn(async move {
        if let Ok(refs) =
            agol_data::process_references_only(client_bg.clone(), token_bg, items_bg).await
        {
            let _ = references_tx.send(refs).await;
        }
    });

    let token_users = Arc::clone(&config.access_token);
    let org_id_users = config.org_info.org_id.clone();

    tokio::spawn(async move {
        if let Ok(refs) = agol::fetch_org_users(&client.clone(), &token_users, &org_id_users).await
        {
            let _ = users_tx.send(refs).await;
        }
    });

    let mut app = ui::init_state(agol.clone(), config.clone());

    while app.state.running {
        terminal.draw(|frame| ui::ui(frame, &mut app))?;

        if let Ok(args_query) = cli_args_rx.try_recv() {
            app.state.queries.push(args_query);
        }

        if let Ok(mut refs) = references_rx.try_recv() {
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

        if let Ok(users) = users_rx.try_recv() {
            app.agol.users = users;
            app.state.users_loading = false;
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
