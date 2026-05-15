use agol::models::{ArcGISOrgInfo, ArcGISReferences, ArcGISSearchResults, Users};
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;

use agol::ArcGISAccessToken;
use ratatui::widgets::{ListState, TableState};

#[derive(Debug)]
pub struct App<'a> {
    pub agol: Agol<'a>,
    pub config: Config,
    pub state: State,
}

#[derive(Debug, Default)]
pub struct State {
    pub agol_content_widget_state: ListState,
    pub reference_table_state: TableState,
    pub broken_connections_state: TableState,
    pub username_state: TableState,
    pub focused_widget: FocusedWidget,
    pub user_input: UserInput,
    pub search_type: SearchType,
    pub input_mode: InputMode,
    pub items_per_username: HashMap<String, u16>,
    pub cli_input: Args,
    pub errors: Option<Errors>,
    pub queries: Vec<String>,
    pub running: bool,
    pub references_loading: bool,
    pub users_loading: bool,
    pub search_popup: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub org_info: ArcGISOrgInfo,
    pub access_token: Arc<ArcGISAccessToken>,
}

#[derive(Debug, Clone)]
pub struct Agol<'a> {
    pub agol_content: Vec<&'a ArcGISSearchResults>,
    pub cached_agol_content: Vec<&'a ArcGISSearchResults>,
    pub references: ArcGISReferences,
    pub users: Vec<Users>,
}

#[derive(Debug, Default, PartialEq)]
pub enum FocusedWidget {
    #[default]
    TopList,
    BottomTable,
    BrokenConnections,
}

#[derive(Debug, Default)]
pub struct UserInput {
    pub input: String,
    pub character_index: usize,
    pub highlight_range: Option<(usize, usize)>,
}

#[derive(Debug, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Debug, Default, PartialEq)]
pub enum SearchType {
    #[default]
    Title,
    Owner,
    Id,
}

#[derive(Debug)]
pub enum Errors {
    NoAccessToken,
    // TODO fetch all org usernames
    //TODO create third widget and display if email not in org usernames
    InvalidUserInput,
}

#[derive(Parser, Debug, Clone, Default)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Email of the user to search
    #[arg(short, long)]
    pub email: Option<String>,

    /// Search term to filter results
    #[arg(short, long)]
    pub search: Option<String>,
}

#[derive(Debug)]
pub enum CliArgsFilter {
    Email,
    SearchTerm,
    Both,
    None,
}
