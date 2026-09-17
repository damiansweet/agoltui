use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table};

use crate::models::{AgolItemIssue, State};

// ERROR WIDGETS

pub fn authentication_error_widget(error: &str) -> Paragraph<'static> {
    Paragraph::new(format!(
        "Could not sign in to ArcGIS Online\n{error}\nPress 'q' to quit"
    ))
    .block(Block::bordered().title("Authentication Error"))
    .style(Style::new().red())
    .alignment(Alignment::Center)
}

pub fn invalid_user_input_widget() -> Paragraph<'static> {
    Paragraph::new("Query must be between 3-50 characters")
        .block(Block::bordered().title("Error"))
        .style(Style::new().red())
        .alignment(Alignment::Center)
}

// SUCCESS WIDGETS
pub fn search_by_user_input_widget(app_state: &State, widget_title: &str) -> Paragraph<'static> {
    Paragraph::new(app_state.user_input.input.clone())
        .block(Block::bordered().title(format!("Search by {}", widget_title)))
}

pub fn structure_mismatches_widget(items: &[AgolItemIssue]) -> Table<'static> {
    let rows = items.iter().map(|issue| {
        Row::new(vec![
            Cell::from(issue.item.id.clone()),
            Cell::from(issue.item.title.clone()),
            Cell::from(issue.item.item_type.clone()),
            Cell::from(issue.item.owner.clone()),
            Cell::from(issue.reason.clone()),
        ])
    });

    Table::new(
        rows,
        [
            Constraint::Percentage(18),
            Constraint::Percentage(22),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(30),
        ],
    )
    .column_spacing(1)
    .style(Style::new().light_red())
    .highlight_symbol(">>")
    .header(Row::new(["Item ID", "Title", "Type", "Owner", "Reason"]))
    .footer(Row::new([
        Cell::from("<Esc> Back").style(Color::LightYellow)
    ]))
    .block(
        Block::bordered()
            .title(format!("Unexpected Item Structure ({})", items.len()))
            .title_alignment(Alignment::Center),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use agol::models::ArcGISSearchResults;
    use ratatui::{Terminal, backend::TestBackend};

    fn render_paragraph(widget: Paragraph<'static>) -> String {
        let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(widget, frame.area()))
            .unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn error_widgets_explain_the_problem() {
        let auth_error = render_paragraph(authentication_error_widget("Invalid credentials"));
        assert!(auth_error.contains("Could not sign in"));
        assert!(auth_error.contains("Invalid credentials"));
        assert!(auth_error.contains("Press 'q' to quit"));

        let input_error = render_paragraph(invalid_user_input_widget());
        assert!(input_error.contains("Query must be between 3-50 characters"));
    }

    #[test]
    fn search_widget_displays_title_and_current_input() {
        let mut state = crate::utils::default_app_state();
        state.user_input.input = "roads".to_string();

        let screen = render_paragraph(search_by_user_input_widget(&state, "Keyword"));

        assert!(screen.contains("Search by Keyword"));
        assert!(screen.contains("roads"));
    }

    #[test]
    fn structure_mismatch_widget_displays_item_information() {
        let issue = AgolItemIssue {
            item: ArcGISSearchResults {
                id: "item-id".to_string(),
                owner: "owner".to_string(),
                org_id: "org-id".to_string(),
                created: 0,
                is_org_item: true,
                modified: 0,
                guid: None,
                name: None,
                title: "Unexpected item".to_string(),
                item_type: "Web Map".to_string(),
                description: None,
                tags: Vec::new(),
                snippet: None,
                url: None,
                access: "private".to_string(),
            },
            reason: "missing field".to_string(),
        };
        let mut terminal = Terminal::new(TestBackend::new(120, 5)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(structure_mismatches_widget(&[issue]), frame.area()))
            .unwrap();

        let screen = terminal.backend().to_string();
        assert!(screen.contains("Unexpected Item Structure (1)"));
        assert!(screen.contains("item-id"));
        assert!(screen.contains("Unexpected item"));
        assert!(screen.contains("Web Map"));
        assert!(screen.contains("owner"));
        assert!(screen.contains("missing field"));
    }
}
