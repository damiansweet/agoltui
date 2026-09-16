use agol::{AgolItemType, ArcGISAccessToken, ArcGISOrgInfo, ArcGISReferences, ArcGISSearchResults};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct ItemIssue {
    pub item: ArcGISSearchResults,
    pub reason: String,
}

#[derive(Debug)]
pub struct LoadedData {
    pub org: ArcGISOrgInfo,
    pub items: Vec<ArcGISSearchResults>,
    pub users: Vec<agol::models::Users>,
    pub references: ArcGISReferences,
    pub structure_issues: Vec<ItemIssue>,
}

#[derive(Debug)]
pub enum LoadEvent {
    Status(String),
    Ready(Box<LoadedData>),
    Failed(String),
}

pub fn spawn_loader(sender: UnboundedSender<LoadEvent>) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = sender.send(LoadEvent::Failed(format!(
                    "Could not start the background runtime: {error}"
                )));
                return;
            }
        };

        runtime.block_on(async move {
            if let Err(error) = load_all(&sender).await {
                let _ = sender.send(LoadEvent::Failed(error));
            }
        });
    });
}

async fn load_all(sender: &UnboundedSender<LoadEvent>) -> Result<(), String> {
    status(sender, "Authenticating with ArcGIS Online…");
    let client = Arc::new(reqwest::Client::new());
    let token = Arc::new(
        agol::fetch_oauth2_agol_token(&client)
            .await
            .map_err(|error| format!("Authentication failed: {error}"))?,
    );

    status(sender, "Loading organization information…");
    let org = agol::fetch_org_info(&client, &token)
        .await
        .map_err(|error| format!("Could not load organization information: {error}"))?;

    status(sender, "Loading organization content…");
    let total = agol::fetch_agol_content_total_count(&client, &token, &org.org_id)
        .await
        .map_err(|error| format!("Could not count organization content: {error}"))?;
    let items =
        agol::fetch_all_agol_content(Arc::clone(&client), Arc::clone(&token), total, &org.org_id)
            .await
            .map_err(|error| format!("Could not load organization content: {error}"))?;

    status(
        sender,
        &format!("Analyzing references for {} items…", items.len()),
    );
    let users_future = agol::fetch_org_users(Arc::clone(&client), Arc::clone(&token), &org.org_id);
    let references_future = process_references(client, token, items.clone());
    let (users, (mut references, structure_issues)) =
        tokio::try_join!(users_future, references_future)
            .map_err(|error| format!("Could not complete organization analysis: {error}"))?;

    references.broken_connections = find_broken_connections(&items, &references);

    let _ = sender.send(LoadEvent::Ready(Box::new(LoadedData {
        org,
        items,
        users,
        references,
        structure_issues,
    })));
    Ok(())
}

fn status(sender: &UnboundedSender<LoadEvent>, message: &str) {
    let _ = sender.send(LoadEvent::Status(message.to_string()));
}

async fn process_references(
    client: Arc<reqwest::Client>,
    token: Arc<ArcGISAccessToken>,
    items: Vec<ArcGISSearchResults>,
) -> agol::Result<(ArcGISReferences, Vec<ItemIssue>)> {
    let mut references = ArcGISReferences::default();
    let mut structure_issues = Vec::new();

    let mut tasks = stream::iter(items)
        .map(|item| {
            let task_item = item.clone();
            let client = Arc::clone(&client);
            let token = Arc::clone(&token);
            async move {
                let task = tokio::spawn(async move {
                    let item_type = AgolItemType::try_from(task_item.item_type.as_str())
                        .map_err(|reason| reason.to_string())?;
                    agol::fetch_per_agol_item_type(&client, &token, &task_item, Ok(item_type))
                        .await
                        .map_err(|error| error.to_string())
                });
                (item, task.await)
            }
        })
        .buffer_unordered(64);

    while let Some((item, result)) = tasks.next().await {
        match result {
            Ok(Ok(item_references)) => {
                for (id, dependents) in item_references.lookup {
                    references.lookup.entry(id).or_default().extend(dependents);
                }
            }
            Ok(Err(reason)) => structure_issues.push(ItemIssue { item, reason }),
            Err(join_error) => structure_issues.push(ItemIssue {
                item,
                reason: panic_reason(join_error),
            }),
        }
    }

    structure_issues.sort_by(|left, right| left.item.title.cmp(&right.item.title));
    Ok((references, structure_issues))
}

fn panic_reason(join_error: tokio::task::JoinError) -> String {
    if !join_error.is_panic() {
        return format!("Reference task failed: {join_error}");
    }

    let payload = match join_error.try_into_panic() {
        Ok(payload) => payload,
        Err(error) => return format!("Reference task failed: {error}"),
    };
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "Item data did not match the expected structure".to_string(),
        },
    }
}

fn find_broken_connections(
    items: &[ArcGISSearchResults],
    references: &ArcGISReferences,
) -> HashSet<ArcGISSearchResults> {
    let valid_ids: HashSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
    references
        .lookup
        .iter()
        .filter(|(id, _)| !valid_ids.contains(id.as_str()))
        .flat_map(|(_, dependents)| dependents.iter().cloned())
        .collect()
}

pub fn owner_counts(items: &[ArcGISSearchResults]) -> Vec<(String, usize)> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item.owner.clone()).or_insert(0) += 1;
    }
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|left, right| left.0.to_lowercase().cmp(&right.0.to_lowercase()));
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, owner: &str) -> ArcGISSearchResults {
        ArcGISSearchResults {
            id: id.to_string(),
            owner: owner.to_string(),
            org_id: "org".to_string(),
            created: 0,
            is_org_item: true,
            modified: 0,
            guid: None,
            name: None,
            title: id.to_string(),
            item_type: "Web Map".to_string(),
            description: None,
            tags: Vec::new(),
            snippet: None,
            url: None,
            access: "private".to_string(),
        }
    }

    #[test]
    fn owner_totals_are_sorted_and_counted() {
        let counts = owner_counts(&[
            item("one", "zoe"),
            item("two", "Alice"),
            item("three", "zoe"),
        ]);
        assert_eq!(counts, [("Alice".to_string(), 1), ("zoe".to_string(), 2)]);
    }

    #[test]
    fn broken_connections_return_dependent_items() {
        let dependent = item("app", "alice");
        let mut references = ArcGISReferences::default();
        references.lookup.insert(
            "missing-source".to_string(),
            HashSet::from([dependent.clone()]),
        );

        let broken = find_broken_connections(std::slice::from_ref(&dependent), &references);
        assert_eq!(broken, HashSet::from([dependent]));
    }
}
