use crate::models::{
    App, Args, CliArgsFilter, FocusedWidget, InputMode, SearchType, State, UserInput,
};
use agol::models::{ArcGISSearchResults, Users};
use clap::Parser;
use ratatui::widgets::{ListItem, ListState, TableState};
use std::collections::{HashMap, HashSet};

pub fn filter_layer_no_references(state: &mut App) {
    let mut no_reference_ids = Vec::new();
    for (k, v) in &state.agol.references.lookup {
        if v.is_empty() {
            no_reference_ids.push(k)
        }
    }

    // combines queries if one already exists
    state.agol.agol_content = state
        .agol
        .agol_content
        .iter()
        .filter(|c| {
            no_reference_ids.contains(&&c.id.clone()) && c.item_type != "Service Definition"
        })
        .copied()
        .collect();
}

//TODO call this from action not UI
pub fn get_layer_references(id: &str, app: &App) -> HashSet<ArcGISSearchResults> {
    if let Some(references) = app.agol.references.lookup.get(id) {
        references.clone()
    } else {
        HashSet::new()
    }
}

pub fn clear_user_input(app_state: &mut State) {
    app_state.user_input.input.clear();
}
pub fn reset_user_input_char_index(app_state: &mut State) {
    app_state.user_input.character_index = 0;
}

pub fn disable_search_popup(app_state: &mut State) {
    app_state.search_popup = false;
}

pub fn filter_usernames_by_user_input<'a>(
    app_state: &State,
    users: &'a [Users],
) -> Vec<ListItem<'a>> {
    users
        .iter()
        .filter(|u| {
            u.username
                .to_lowercase()
                .contains(&app_state.user_input.input.to_lowercase())
        })
        .map(|u| ListItem::new(u.username.as_str()))
        .collect()
}

pub fn filter_title_by_user_input<'a>(
    app_state: &State,
    agol_items: &Vec<&'a ArcGISSearchResults>,
) -> Vec<ListItem<'a>> {
    agol_items
        .iter()
        .filter(|a| {
            a.title
                .to_lowercase()
                .contains(&app_state.user_input.input.to_lowercase())
        })
        .map(|a| ListItem::new(a.title.as_str()))
        .collect()
}

pub fn filter_id_by_user_input<'a>(
    app_state: &State,
    agol_items: &Vec<&'a ArcGISSearchResults>,
) -> Vec<ListItem<'a>> {
    agol_items
        .iter()
        .filter(|a| a.id.contains(&app_state.user_input.input))
        .map(|a| ListItem::new(format!("{} | {}", a.id.as_str(), a.title.as_str())))
        .collect()
}

pub fn default_app_state() -> State {
    State {
        agol_content_widget_state: ListState::default().with_selected(Some(0)),
        reference_table_state: TableState::default().with_selected(Some(0)),
        broken_connections_state: TableState::default().with_selected(Some(0)),
        structure_mismatches_state: TableState::default().with_selected(Some(0)),
        username_state: TableState::default().with_selected(Some(0)),
        focused_widget: FocusedWidget::default(),
        user_input: UserInput::default(),
        search_type: SearchType::default(),
        input_mode: InputMode::default(),
        items_per_username: HashMap::default(),
        errors: None,
        queries: Vec::default(),
        running: true,
        references_loading: true,
        users_loading: true,
        search_popup: false,
    }
}

pub fn filter_cli_args<'a>(
    agol_items: &'a [ArcGISSearchResults],
    args: &Args,
    filter_type: &CliArgsFilter,
) -> Vec<&'a ArcGISSearchResults> {
    match filter_type {
        CliArgsFilter::Both => agol_items
            .iter()
            .filter(|i| {
                i.owner
                    .to_lowercase()
                    .contains(&args.email.as_ref().unwrap().to_lowercase())
                    && i.title
                        .to_lowercase()
                        .contains(&args.search.as_ref().unwrap().to_lowercase())
            })
            .collect(),
        CliArgsFilter::Email => agol_items
            .iter()
            .filter(|i| {
                i.owner
                    .to_lowercase()
                    .contains(&args.email.as_ref().unwrap().to_lowercase())
            })
            .collect(),
        CliArgsFilter::SearchTerm => agol_items
            .iter()
            .filter(|i| {
                i.title
                    .to_lowercase()
                    .contains(&args.search.as_ref().unwrap().to_lowercase())
            })
            .collect(),
        CliArgsFilter::ItemId => agol_items
            .iter()
            .filter(|i| i.id.contains(args.item_id.as_ref().unwrap()))
            .collect(),
        CliArgsFilter::None => agol_items.iter().collect(),
    }
}

pub async fn build_cli_args_query(args: Args, filter_type: CliArgsFilter) -> Option<String> {
    match filter_type {
        CliArgsFilter::Both => Some(format!(
            "Owner/Username == '{}' &&  Title ILIKE '{}'",
            args.email.unwrap_or_default(),
            args.search.unwrap_or_default()
        )),
        CliArgsFilter::Email => Some(format!(
            "Owner/Username ILIKE '{}'",
            args.email.unwrap_or_default()
        )),
        CliArgsFilter::SearchTerm => {
            Some(format!("Title ILIKE '{}'", args.search.unwrap_or_default()))
        }
        CliArgsFilter::ItemId => Some(format!("Item ID: '{}'", args.item_id.unwrap_or_default())),
        CliArgsFilter::None => None,
    }
}

pub fn check_cli_args() -> (Args, CliArgsFilter) {
    let cli_args = Args::parse();
    let cli_filter = match cli_args {
        Args {
            email: Some(_),
            search: Some(_),
            item_id: None,
        } => CliArgsFilter::Both,

        Args {
            email: Some(_),
            search: None,
            item_id: None,
        } => CliArgsFilter::Email,
        Args {
            email: None,
            search: Some(_),
            item_id: None,
        } => CliArgsFilter::SearchTerm,
        Args {
            email: None,
            search: None,
            item_id: None,
        } => CliArgsFilter::None,
        Args {
            item_id: Some(_), ..
        } => CliArgsFilter::ItemId,
    };

    (cli_args, cli_filter)
} //TODO create test for  default_app_state

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Agol, Config};
    use agol::models::ArcGISReferences;

    fn item(id: &str, title: &str, owner: &str, item_type: &str) -> ArcGISSearchResults {
        ArcGISSearchResults {
            id: id.to_string(),
            owner: owner.to_string(),
            org_id: "org-id".to_string(),
            created: 0,
            is_org_item: true,
            modified: 0,
            guid: None,
            name: None,
            title: title.to_string(),
            item_type: item_type.to_string(),
            description: None,
            tags: Vec::new(),
            snippet: None,
            url: None,
            access: "private".to_string(),
        }
    }

    #[test]
    fn default_state_is_ready_for_initial_loading() {
        let state = default_app_state();

        assert!(state.running);
        assert!(state.references_loading);
        assert!(state.users_loading);
        assert!(!state.search_popup);
        assert_eq!(state.focused_widget, FocusedWidget::TopList);
        assert_eq!(state.agol_content_widget_state.selected(), Some(0));
        assert_eq!(state.reference_table_state.selected(), Some(0));
    }

    #[test]
    fn cli_filters_are_case_insensitive_and_composable() {
        let items = [
            item("roads-1", "Road Closures", "Alice", "Web Map"),
            item("parks-2", "City Parks", "Bob", "Web Map"),
        ];

        let by_owner = filter_cli_args(
            &items,
            &Args {
                email: Some("ALICE".to_string()),
                ..Args::default()
            },
            &CliArgsFilter::Email,
        );
        assert_eq!(
            by_owner
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["roads-1"]
        );

        let by_both = filter_cli_args(
            &items,
            &Args {
                email: Some("alice".to_string()),
                search: Some("ROAD".to_string()),
                item_id: None,
            },
            &CliArgsFilter::Both,
        );
        assert_eq!(by_both.len(), 1);

        let by_id = filter_cli_args(
            &items,
            &Args {
                item_id: Some("parks".to_string()),
                ..Args::default()
            },
            &CliArgsFilter::ItemId,
        );
        assert_eq!(by_id[0].title, "City Parks");
    }

    #[test]
    fn no_reference_filter_excludes_service_definitions() {
        let items = [
            item("empty", "Unused layer", "alice", "Feature Service"),
            item("used", "Used layer", "alice", "Feature Service"),
            item(
                "definition",
                "Publish source",
                "alice",
                "Service Definition",
            ),
        ];
        let content: Vec<_> = items.iter().collect();
        let mut lookup = HashMap::new();
        lookup.insert("empty".to_string(), HashSet::new());
        lookup.insert("used".to_string(), HashSet::from([items[0].clone()]));
        lookup.insert("definition".to_string(), HashSet::new());
        let mut app = App {
            agol: Agol {
                agol_content: content.clone(),
                cached_agol_content: content,
                references: ArcGISReferences {
                    lookup,
                    ..ArcGISReferences::default()
                },
                ..Agol::default()
            },
            config: Config::default(),
            state: default_app_state(),
        };

        filter_layer_no_references(&mut app);

        assert_eq!(app.agol.agol_content.len(), 1);
        assert_eq!(app.agol.agol_content[0].id, "empty");
    }

    #[test]
    fn interactive_option_filters_match_partial_input() {
        let items = [
            item("roads-1", "Road Closures", "alice", "Web Map"),
            item("parks-2", "City Parks", "bob", "Web Map"),
        ];
        let refs: Vec<_> = items.iter().collect();
        let mut state = default_app_state();
        state.user_input.input = "ROAD".to_string();
        assert_eq!(filter_title_by_user_input(&state, &refs).len(), 1);

        state.user_input.input = "parks".to_string();
        assert_eq!(filter_id_by_user_input(&state, &refs).len(), 1);

        let users = [
            Users {
                username: "alice.admin".to_string(),
                ..Users::default()
            },
            Users {
                username: "bob.editor".to_string(),
                ..Users::default()
            },
        ];
        state.user_input.input = "ADMIN".to_string();
        assert_eq!(filter_usernames_by_user_input(&state, &users).len(), 1);
    }

    #[tokio::test]
    async fn cli_query_text_covers_each_filter_type() {
        let email = Args {
            email: Some("alice".to_string()),
            ..Args::default()
        };
        assert_eq!(
            build_cli_args_query(email, CliArgsFilter::Email)
                .await
                .as_deref(),
            Some("Owner/Username ILIKE 'alice'")
        );

        let search = Args {
            search: Some("roads".to_string()),
            ..Args::default()
        };
        assert_eq!(
            build_cli_args_query(search, CliArgsFilter::SearchTerm)
                .await
                .as_deref(),
            Some("Title ILIKE 'roads'")
        );

        assert!(
            build_cli_args_query(Args::default(), CliArgsFilter::None)
                .await
                .is_none()
        );
    }
}
