use crate::ui::Args;
use agol::models::ArcGISSearchResults;
use chrono::Local;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub fn format_email(email: &str) -> &str {
    if email.eq_ignore_ascii_case("damian.sweet@cityoflonetree.com") {
        "Damian.Sweet@cityoflonetree.com"
    } else if email.eq_ignore_ascii_case("courtland.langley@cityoflonetree.com") {
        "courtland.langley@cityoflonetree.com"
    } else {
        email
    }
}

pub fn read_last_sync() -> String {
    let file = std::fs::read_to_string("data/last_sync.txt");

    match file {
        Ok(file_contents) => {
            if file_contents.len() > 0 {
                // let sync_time: Vec<&str> = file.split("\n").collect();

                // let sync_time = sync_time[0].to_string();
                let sync_time = file_contents.trim();
                sync_time.to_string()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}
//TODO display feature layer info that has the most references

//TODO test how many results come from this
pub fn filter_layer_no_references() -> Vec<ArcGISSearchResults> {
    if let Ok(file) = std::fs::read_to_string("data/all_layers_with_web_maps.json") {
        let layers_with_references: HashMap<String, Vec<String>> =
            serde_json::from_str(&file).expect("unable to convert json file to HashMap");
        let layer_with_references: Vec<String> = layers_with_references
            .into_iter()
            .filter(|(_layer, references)| !references.is_empty())
            .map(|(layer, _references)| layer)
            .collect();

        if let Ok(file) = std::fs::read_to_string("data/all_agol_content.json") {
            let all_content: Vec<ArcGISSearchResults> =
                serde_json::from_str(&file).expect("unable to convert json to struct");
            all_content
                .into_iter()
                .filter(|content| {
                    !layer_with_references.contains(&content.id)
                        && content.item_type == "Feature Service"
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

//TODO call this from action not UI
pub fn get_layer_references(id: &str) -> Vec<String> {
    let file = std::fs::read_to_string("data/all_layers_with_web_maps.json");

    let file_string = match file {
        Ok(file_string) => file_string,
        Err(_) => String::from(""),
    };

    let layer_references: HashMap<String, Vec<String>> =
        serde_json::from_str(&file_string).unwrap_or_default();

    if let Some(references) = layer_references.get(id) {
        // references.clone()
        references
            .into_iter()
            .map(|item| format!("https://cityoflonetree.maps.arcgis.com/home/item.html?id={item}"))
            .collect()
    } else {
        Vec::new()
    }
}

pub fn load_all_content_from_file() -> Result<Vec<ArcGISSearchResults>, Box<dyn std::error::Error>>
{
    let data = std::fs::read_to_string("data/all_agol_content.json")?;

    let data = serde_json::from_str(&data)?;
    Ok(data)
}
pub fn refresh_data(
    client: &reqwest::blocking::Client,
    access_token: &agol::models::ArcGISAccessToken,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let all_agol_content = agol::fetch_all_agol_content_blocking(client, access_token)?;
    let all_agol_content_path = Path::new("data/all_agol_content.json");
    agol::pretty_write_all_agol_content_to_file(&all_agol_content_path, &all_agol_content)?;

    let web_maps = agol::filter_web_maps(&all_agol_content);
    let web_map_ids = agol::extract_agol_ids(&web_maps);

    let all_layers_with_web_maps =
        agol::fetch_layers_for_all_web_maps_blocking(client, access_token, &web_map_ids)?;
    //write all layers with web maps to json file

    let layers_with_web_map_path = Path::new("data/all_layers_with_web_maps.json");
    pretty_write_all_layers_with_web_maps_to_file(
        layers_with_web_map_path,
        all_layers_with_web_maps,
    )?;

    let current_time = get_current_time()?;

    fs::write("data/last_sync.txt", current_time)?;

    Ok(())
}
pub fn pretty_write_all_layers_with_web_maps_to_file(
    file_path: &Path,
    all_layers: HashMap<String, HashSet<String>>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(file_path).expect("failed to create file");
    let writer = std::io::BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &all_layers)?;

    Ok(())
}

fn get_current_time() -> std::result::Result<String, Box<dyn std::error::Error>> {
    let dt = Local::now();
    let date_string = dt.to_rfc2822();
    Ok(date_string)
}
