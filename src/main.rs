use crate::{
    errors::AppError,
    models::{App, Errors},
};
use agol::models::{ArcGISReferences, ArcGISSearchResults, Users};
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

use std::collections::HashSet;
use std::io::{self, Write};

use crate::models::{Agol, Config};

mod action;
mod agol_data;
mod auth;
mod errors;
mod models;
mod ui;
mod utils;
mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (errors_tx, mut errors_rx) = tokio::sync::mpsc::unbounded_channel::<Errors>();
    let (cli_args_tx, mut cli_args_rx) = tokio::sync::mpsc::unbounded_channel();
    let (users_tx, mut users_rx) = tokio::sync::mpsc::unbounded_channel();
    let (references_tx, mut references_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut app = ui::init_state(Agol::default(), Config::default());
    let mut agol_items: Vec<ArcGISSearchResults> = vec![];

    let client = Arc::new(auth::http_client().map_err(io::Error::other)?);
    let credentials = match auth::application_credentials() {
        Some(credentials) => credentials,
        None => prompt_for_user_credentials()?,
    };
    match auth::authenticate(&client, credentials).await {
        Ok(authenticated) => {
            if let Some(username) = &authenticated.username {
                println!("Verified ArcGIS Online account: {username}");
            }
            let access_token = authenticated.token;
            let config = Config {
                org_info: agol::fetch_org_info(&client, &access_token).await?,
                access_token: Arc::new(access_token.clone()),
            };

            let total_agol_count = agol::fetch_agol_content_total_count(
                &client,
                &access_token,
                &config.org_info.org_id,
            )
            .await?;
            agol_items = agol_data::fetch_agol_data(
                Arc::clone(&client),
                Arc::clone(&config.access_token),
                total_agol_count,
                &config.org_info.org_id,
            )
            .await?;

            let (cli_args, cli_filter) = utils::check_cli_args();

            let agol = Agol {
                agol_content: utils::filter_cli_args(&agol_items, &cli_args, &cli_filter),
                cached_agol_content: agol_items.iter().collect(),
                references: ArcGISReferences::default(),
                structure_mismatches: Vec::new(),
                users: vec![Users::default()],
            };

            // Sets initial query in state if cli args are provided
            {
                let cli_args = cli_args.clone();

                tokio::spawn(async move {
                    if let Some(query) = utils::build_cli_args_query(cli_args, cli_filter).await {
                        let _ = cli_args_tx.send(query);
                    }
                });
            }

            // Sets references state
            {
                let client = Arc::clone(&client);
                let token = Arc::clone(&config.access_token);
                let agol_items_bg = agol_items.clone();
                let valid_agol_item_ids = agol::extract_item_ids(&agol_items.clone());

                //TODO cleanup extra clones if possible
                tokio::spawn(async move {
                    if let Ok(processed) =
                        agol_data::process_references_only(client, token, agol_items_bg).await
                    {
                        let mut refs = processed.references;
                        let mut broken_connections: HashSet<ArcGISSearchResults> = HashSet::new();

                        for (k, v) in &refs.lookup {
                            if !valid_agol_item_ids.contains(k) {
                                for j in v {
                                    broken_connections.insert(j.clone());
                                }
                            }
                        }
                        refs.broken_connections = broken_connections;
                        let _ = references_tx.send((refs, processed.structure_mismatches));
                    }
                });
            }

            // Sets users state
            {
                let client = Arc::clone(&client);
                let token = Arc::clone(&config.access_token);
                let org_id = config.org_info.org_id.clone();

                tokio::spawn(async move {
                    if let Ok(refs) = agol::fetch_org_users(client, token, &org_id).await {
                        let _ = users_tx.send(refs);
                    }
                });
            }

            app = ui::init_state(agol, config);
        }
        Err(error) => {
            tokio::spawn(async move {
                let _ = errors_tx.send(Errors::Authentication(error));
            });
        }
    };

    let mut terminal = ratatui::init();

    run(
        &mut terminal,
        &mut app,
        &mut errors_rx,
        &mut cli_args_rx,
        &mut references_rx,
        &mut users_rx,
    )
    .await?;

    Ok(())
}

fn prompt_for_user_credentials() -> color_eyre::Result<auth::Credentials> {
    println!(
        "ArcGIS OAuth application credentials are not set. Sign in with an ArcGIS Online account."
    );
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err(color_eyre::eyre::eyre!("ArcGIS username cannot be empty"));
    }
    let password = rpassword::prompt_password("Password: ")?;
    if password.is_empty() {
        return Err(color_eyre::eyre::eyre!("ArcGIS password cannot be empty"));
    }
    Ok(auth::Credentials::User { username, password })
}

async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    errors_rx: &mut UnboundedReceiver<Errors>,
    cli_args_rx: &mut UnboundedReceiver<String>,
    references_rx: &mut UnboundedReceiver<(ArcGISReferences, Vec<models::AgolItemIssue>)>,
    users_rx: &mut UnboundedReceiver<Vec<Users>>,
) -> std::io::Result<()> {
    while app.state.running {
        terminal.draw(|frame| ui::ui(frame, app))?;
        if let Ok(errors) = errors_rx.try_recv() {
            app.state.errors = Some(errors);
        }

        if let Ok(args_query) = cli_args_rx.try_recv() {
            app.state.queries.push(args_query);
        }

        if let Ok((refs, structure_mismatches)) = references_rx.try_recv() {
            app.agol.references = refs;
            app.agol.structure_mismatches = structure_mismatches;
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
            let action = action::handle_key(&app.state, key.code);
            action::handle_action(app, action).await;
        }
    }

    ratatui::restore();
    Ok(())
}
