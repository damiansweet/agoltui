use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};

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
