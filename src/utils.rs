use crate::ui::{Args, UiState};
use agol::models::{ArcGISReferences, ArcGISSearchResults};
use chrono::Local;
use std::collections::{HashMap, HashSet};
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

//TODO display feature layer info that has the most references

//TODO test how many results come from this
pub fn filter_layer_no_references(state: &mut UiState) -> Vec<&ArcGISSearchResults> {
    let mut no_reference_ids = Vec::new();
    for (k, v) in &state.references_lookup.lookup {
        if v.is_empty() {
            no_reference_ids.push(k)
        }
    }

    state
        .agol_content
        .iter()
        .filter(|c| no_reference_ids.contains(&&c.id.clone()))
        .collect()
}

//TODO call this from action not UI
pub fn get_layer_references(id: &str, ui_state: &UiState) -> HashSet<ArcGISSearchResults> {
    if let Some(references) = ui_state.references_lookup.lookup.get(id) {
        references.clone()
    } else {
        HashSet::new()
    }
}

pub fn load_all_content_from_file() -> Result<Vec<ArcGISSearchResults>, Box<dyn std::error::Error>>
{
    let data = std::fs::read_to_string("data/all_agol_content.json")?;

    let data = serde_json::from_str(&data)?;
    Ok(data)
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
