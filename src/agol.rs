use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const ORG_ID: &str = "M6RDkiPo9JtEo7N6";

#[derive(Debug)]
pub enum AgolItemType {
    Form,
    Style,
    Solution,
    FileGeodatabase,
    MicrosoftWord,
    GeocodingService,
    ServiceDefinition,
    VectorTileService,
    WebScene,
    HubSiteApplication,
    HubPage,
    Application,
    WebMappingApplication,
    FeatureCollection,
    GeoJson,
    WebMap,
    Dashboard,
    Shapefile,
    AdministrativeReport,
    MicrosoftExcel,
    GroupLayer,
    Image,
    DesktopStyle,
    WebExperienceTemplate,
    WebExperience,
    CSV,
    Notebook,
    FeatureService,
    GeoprocessingService,
    PDF,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ArcGISSearchResults {
    pub id: String,
    pub owner: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
    pub created: u64,
    #[serde(rename = "isOrgItem")]
    pub is_org_item: bool,
    pub modified: u64,
    pub guid: Option<String>,
    pub name: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub snippet: Option<String>,
    pub url: Option<String>,
    pub access: String,
}

#[derive(Deserialize, Debug)]
pub struct ArcGISAccessToken {
    pub access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct ArcGISSearchResponse {
    pub total: u32,
    #[serde(rename = "nextStart")]
    pub next_start: i32,
    pub results: Vec<ArcGISSearchResults>,
}

pub fn fetch_oath2_agol_token_blocking(client: &Client) -> Result<ArcGISAccessToken> {
    let url = "https://www.arcgis.com/sharing/rest/oauth2/token";
    let form_params = [
        ("client_id", env!("ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID")),
        (
            "client_secret",
            env!("ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET"),
        ),
        ("grant_type", "client_credentials"),
        ("f", "json"),
    ];

    let resp = client.post(url).form(&form_params).send()?;

    let resp: ArcGISAccessToken = resp.json()?;

    Ok(resp)
}

pub fn fetch_agol_content_total(client: &Client, access_token: &ArcGISAccessToken) -> Result<u32> {
    let url = "https://cityoflonetree.maps.arcgis.com/sharing/rest/search";
    let query_params = [
        ("f", "json"),
        ("q", &format!("orgid:{ORG_ID}")),
        ("num", "0"),
        ("token", &access_token.access_token),
    ];

    let resp = client.get(url).query(&query_params).send()?;

    let resp: ArcGISSearchResponse = resp.json()?;

    // println!("{:#?}", resp.text());
    Ok(resp.total)
}
