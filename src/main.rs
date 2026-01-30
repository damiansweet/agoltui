use crossterm::event::{self, Event, KeyCode};
// use ratatui::DefaultTerminal;
use ratatui::{
    Frame,
    layout::Alignment,
    widgets::{Block, Borders, Paragraph},
};

struct App {
    should_quit: bool,
}

fn update(mut app: App, key: char) -> App {
    match key {
        'q' => app.should_quit = true,
        _ => {}
    };
    app
}

fn ui(frame: &mut Frame, _app: &App) {
    let area = frame.area();

    let widget = Paragraph::new("Hello There Damian!")
        .block(
            Block::default()
                .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(widget, area);
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let mut state = App { should_quit: false };

    while !state.should_quit {
        terminal.draw(|frame| ui(frame, &state))?;

        if let Event::Key(key) = event::read()? {
            if let KeyCode::Esc = key.code {
                state = update(state, 'q');
            }
        }
    }

    ratatui::restore();
    Ok(())
}
