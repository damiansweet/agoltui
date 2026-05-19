use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};

use crate::models::State;

// ERROR WIDGETS

pub fn no_access_token_error_widget() -> Paragraph<'static> {
    Paragraph::new("Invalid Access Token\nPress 'q' to quit")
        .block(Block::bordered().title("Error"))
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
