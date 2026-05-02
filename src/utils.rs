use crate::ui::UiState;
use agol::models::ArcGISSearchResults;
use std::collections::HashSet;

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
        .filter(|c| {
            no_reference_ids.contains(&&c.id.clone()) && c.item_type != "Service Definition"
        })
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

pub fn previous_word_starting_index(word: &str) -> usize {
    //TODO first get length of previous word
    // TODO delete previous word from user_input
    // TODO set cursor back length of previous word
    let Some(previous_word) = word.split(" ").last() else {
        return 0;
    };
    previous_word.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_previous_word_index() {
        assert_eq!(previous_word_starting_index("hello"), 5);
        assert_eq!(previous_word_starting_index("hello Rustacean"), 9);
    }
}
