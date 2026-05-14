#![allow(clippy::pedantic)]
use crate::AppError;
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
