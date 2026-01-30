use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
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
    // let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.area());

    let widget_top = Paragraph::new("Hello There Damian!")
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    let widget_bottom = Paragraph::new("Hello There AGAIN Damian!")
        .block(
            Block::default()
                // .title("Functional Ratatui")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    frame.render_widget(widget_top, layout[0]);

    frame.render_widget(widget_bottom, layout[1]);
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let mut state = App { should_quit: false };

    while !state.should_quit {
        terminal.draw(|frame| ui(frame, &state))?;

        if let Event::Key(key) = event::read()? {
            // if let KeyCode::Esc = key.code {
            if let KeyCode::Char('q') = key.code {
                state = update(state, 'q');
            }
        }
    }

    ratatui::restore();
    Ok(())
}
