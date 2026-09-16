use crate::{AppError, models::AgolItemIssue};
use agol::{AgolItemType, ArcGISAccessToken, ArcGISReferences, ArcGISSearchResults};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub async fn fetch_agol_data(
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

    Ok(results)
}

pub async fn process_references_only(
    client: Arc<reqwest::Client>,
    access_token: Arc<ArcGISAccessToken>,
    results: Vec<ArcGISSearchResults>,
) -> Result<ProcessedReferences, AppError> {
    let mut references = ArcGISReferences {
        lookup: HashMap::new(),
        broken_connections: HashSet::new(),
    };

    let mut structure_mismatches = Vec::new();
    let mut stream_of_futures = stream::iter(results)
        .map(|item| {
            let item_for_task = item.clone();
            let client = Arc::clone(&client);
            let access_token = Arc::clone(&access_token);
            async move {
                let task = tokio::spawn(async move {
                    let item_type = AgolItemType::try_from(item_for_task.item_type.as_str())
                        .map_err(|error| error.to_string())?;
                    agol::fetch_per_agol_item_type(
                        &client,
                        &access_token,
                        &item_for_task,
                        Ok(item_type),
                    )
                    .await
                    .map_err(|error| error.to_string())
                });
                (item, task.await)
            }
        })
        .buffer_unordered(100);

    while let Some((item, task_result)) = stream_of_futures.next().await {
        match task_result {
            Ok(Ok(r)) => {
                for (k, v) in r.lookup {
                    references.lookup.entry(k).or_default().extend(v);
                }
            }
            Ok(Err(reason)) => structure_mismatches.push(AgolItemIssue { item, reason }),
            Err(join_error) => structure_mismatches.push(AgolItemIssue {
                item,
                reason: panic_reason(join_error),
            }),
        }
    }

    structure_mismatches.sort_by(|a, b| a.item.title.cmp(&b.item.title));

    Ok(ProcessedReferences {
        references,
        structure_mismatches,
    })
}

#[derive(Debug)]
pub struct ProcessedReferences {
    pub references: ArcGISReferences,
    pub structure_mismatches: Vec<AgolItemIssue>,
}

fn panic_reason(join_error: tokio::task::JoinError) -> String {
    if !join_error.is_panic() {
        return format!("Reference processing task failed: {join_error}");
    }

    let panic = join_error
        .try_into_panic()
        .expect("is_panic was checked before extracting the panic payload");
    match panic.downcast::<String>() {
        Ok(message) => *message,
        Err(panic) => match panic.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "Item data did not match the expected structure".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search_result(item_type: &str) -> ArcGISSearchResults {
        ArcGISSearchResults {
            id: "item-id".to_string(),
            owner: "owner".to_string(),
            org_id: "org-id".to_string(),
            created: 0,
            is_org_item: true,
            modified: 0,
            guid: None,
            name: None,
            title: "Unexpected item".to_string(),
            item_type: item_type.to_string(),
            description: None,
            tags: Vec::new(),
            snippet: None,
            url: None,
            access: "private".to_string(),
        }
    }

    #[tokio::test]
    async fn preserves_items_with_unknown_types_as_structure_mismatches() {
        let processed = process_references_only(
            Arc::new(reqwest::Client::new()),
            Arc::new(ArcGISAccessToken::default()),
            vec![search_result("Unexpected Type")],
        )
        .await
        .expect("reference processing should continue");

        assert!(processed.references.lookup.is_empty());
        assert_eq!(processed.structure_mismatches.len(), 1);
        assert_eq!(processed.structure_mismatches[0].item.id, "item-id");
        assert_eq!(
            processed.structure_mismatches[0].reason,
            "Unknown item type: Unexpected Type"
        );
    }
}
