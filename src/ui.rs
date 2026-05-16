use agol::models::ArcGISSearchResults;

use crate::helix_keybinds::build_input_spans;
use crate::models::{Agol, App, Config, Errors, FocusedWidget, InputMode, SearchType};
use crate::utils;
use crate::widgets::{invalid_user_input_widget, no_access_token_error_widget};
use ratatui::style::{Color, Style};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Position},
    widgets::{
        Block, Cell, Clear, HighlightSpacing, List, ListDirection, ListItem, Paragraph, Row, Table,
        Wrap,
    },
};

pub fn init_state(agol: Agol, config: Config) -> App {
    App {
        agol,
        config,
        state: utils::default_app_state(),
    }
}

fn selected_item<'a>(
    state: &App,
    items: &Vec<&'a ArcGISSearchResults>,
) -> Option<&'a ArcGISSearchResults> {
    state
        .state
        .agol_content_widget_state
        .selected()
        .and_then(|i| items.get(i))
        .map(|v| &**v)
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    match app.state.errors {
        Some(Errors::NoAccessToken) => {
            frame.render_widget(no_access_token_error_widget(), frame.area())
        }
        Some(Errors::InvalidUserInput) => {
            frame.render_widget(invalid_user_input_widget(), frame.area());
        }
        None => {
            if app.state.search_popup {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Percentage(40),
                        Constraint::Percentage(40),
                        Constraint::Percentage(20),
                    ])
                    .split(frame.area());

                let input_spans = build_input_spans(
                    &app.state.user_input.input,
                    app.state.user_input.character_index,
                    app.state.user_input.highlight_range,
                    &app.state.input_mode,
                );

                let user_input = match app.state.search_type {
                    SearchType::Title => Paragraph::new(input_spans.clone())
                        .block(Block::bordered().title("Search by Keyword")),
                    SearchType::Owner => Paragraph::new(input_spans.clone())
                        .block(Block::bordered().title("Search by Email")),
                    SearchType::Id => Paragraph::new(input_spans.clone())
                        .block(Block::bordered().title("Search by Item Id")),
                };

                let key_binds_widget = Paragraph::new(
                    "Search by Keyword: <F1>\nSearch by Email: <F2>\nSearch by Item Id: <F3>",
                )
                .style(Style::new().light_blue())
                .block(Block::bordered().title("KeyBinds"));

                let valid_users_widget = List::from_iter(utils::extract_usernames(&app.agol.users));

                let input_area = frame.area();
                frame.render_widget(Clear, frame.area());
                frame.render_widget(user_input, layout[0]);
                frame.render_widget(valid_users_widget, layout[1]);
                frame.render_widget(key_binds_widget, layout[2]);

                if matches!(app.state.input_mode, InputMode::Editing) {
                    frame.set_cursor_position(Position::new(
                        input_area.x + app.state.user_input.character_index as u16 + 1,
                        input_area.y + 1,
                    ));
                }
            } else {
                if !app.state.items_per_username.is_empty() {
                    let current_query = app.state.queries.clone();
                    let rows: Vec<Row> = app
                        .state
                        .items_per_username
                        .iter()
                        .map(|(k, v)| Row::new(vec![k.to_string(), v.to_string()]))
                        .collect();

                    let widths = [Constraint::Length(80), Constraint::Length(20)];
                    let username_widget = Table::new(rows, widths)
                        .column_spacing(1)
                        .style(Style::new().blue())
                        .highlight_symbol(">>")
                        .header(Row::new(vec!["Username", "# of Items"]))
                        .footer(Row::new(vec![Cell::new(current_query.join(" && "))]))
                        .block(Block::new().title("Usernames Table"));

                    frame.render_stateful_widget(
                        username_widget,
                        frame.area(),
                        &mut app.state.username_state,
                    )
                } else if app.state.focused_widget == FocusedWidget::BrokenConnections {
                    let broken_connections: Vec<&ArcGISSearchResults> =
                        app.agol.references.broken_connections.iter().collect();
                    let rows: Vec<Row> = broken_connections
                        .iter()
                        .map(|item| {
                            Row::new(vec![
                                item.id.to_string(),
                                item.title.clone(),
                                item.item_type.clone(),
                            ])
                        })
                        .collect();

                    let widths = [
                        Constraint::Percentage(30),
                        Constraint::Percentage(35),
                        Constraint::Percentage(35),
                    ];
                    let broken_connections_widget = Table::new(rows, widths)
                        .column_spacing(1)
                        .style(Style::new().light_blue())
                        .highlight_symbol(">>")
                        .header(Row::new(vec!["Item ID", "Title", "Type"]))
                        .block(Block::new().title("Broken Connections"));

                    frame.render_stateful_widget(
                        broken_connections_widget,
                        frame.area(),
                        &mut app.state.broken_connections_state,
                    )
                } else {
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![
                            Constraint::Percentage(40),
                            Constraint::Percentage(20),
                            Constraint::Percentage(40),
                        ])
                        .split(frame.area());

                    let all_content_ids: Vec<ListItem> = app
                        .agol
                        .agol_content
                        .iter()
                        .map(|item| ListItem::new(item.id.clone()))
                        .collect();

                    let num_list_items = app.agol.agol_content.len();

                    let widget_top = List::new(all_content_ids)
                        .block(
                            Block::bordered()
                                .title_alignment(Alignment::Center)
                                .title(format!("AGOL Content List\t {num_list_items}"))
                                .style(if app.state.focused_widget == FocusedWidget::TopList {
                                    Style::new().bg(Color::DarkGray)
                                } else {
                                    Style::new()
                                }),
                        )
                        .style(Style::new().white())
                        .highlight_symbol(">>")
                        .highlight_spacing(HighlightSpacing::Always)
                        .repeat_highlight_symbol(true)
                        .direction(ListDirection::TopToBottom)
                        .highlight_style(if app.state.focused_widget == FocusedWidget::TopList {
                            Style::new().italic()
                        } else {
                            Style::new()
                        });

                    let selected_title = selected_item(app, &app.agol.agol_content)
                        .map(|item| item.title.as_str())
                        .unwrap_or_default();

                    let selected_item_type = selected_item(app, &app.agol.agol_content)
                        .map(|item| item.item_type.as_str())
                        .unwrap_or_default();

                    let selected_owner = selected_item(app, &app.agol.agol_content)
                        .map(|item| item.owner.as_str())
                        .unwrap_or_default();

                    let queries = &app.state.queries.join(" && ");

                    let layer_info_text = format!(
                        "Title: {selected_title}\nItem Type: {selected_item_type}\nOwner: {selected_owner}\n<j>/<Down> Navigate Down | <k>/<Up> Navigate Up\n<f> filter by username | <0> zero references\nCurrent Query: {queries}"
                    );

                    let widget_center = if app.state.references_loading {
                        Paragraph::new("Loading references...")
                            .block(Block::bordered().title("References"))
                            .style(Style::new().yellow())
                    } else {
                        Paragraph::new(layer_info_text)
                            .wrap(Wrap { trim: true })
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("Layer Info"),
                            )
                            .style(Style::new().white())
                            .alignment(Alignment::Center)
                    };

                    let widget_bottom = if let Some(selected_id) =
                        selected_item(app, &app.agol.agol_content).map(|item| item.id.as_str())
                    {
                        let references = utils::get_layer_references(selected_id, app);
                        let mut sorted_references: Vec<ArcGISSearchResults> = Vec::new();
                        for r in &references {
                            sorted_references.push(r.clone());
                        }
                        sorted_references.sort_by(|a, b| a.title.cmp(&b.title));
                        // references.sort_by(|a, b| a);
                        let header = Row::new(["Index", "Title", "Type", "Url"])
                            .style(Style::new().bold())
                            .bottom_margin(1);

                        let mut rows: Vec<Row> = Vec::new();
                        for (i, r) in sorted_references.into_iter().enumerate() {
                            let url = format!(
                                "{}/home/item.html?id={}",
                                &app.config.org_info.full_url, &r.id
                            );
                            rows.push(Row::new([i.to_string(), r.title, r.item_type, url]));
                        }
                        //TODO conditionally render no references if !sorted_references.is_empty()

                        let selected = app.state.reference_table_state.selected();
                        if selected.is_none()
                            || selected.unwrap_or(0) >= rows.len() && !rows.is_empty()
                        {
                            app.state.reference_table_state.select(Some(0));
                        }

                        let widths = [
                            Constraint::Percentage(5),
                            Constraint::Percentage(25),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .column_spacing(1)
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("References")
                                    .style(
                                        if app.state.focused_widget == FocusedWidget::BottomTable {
                                            Style::new().bg(Color::DarkGray)
                                        } else {
                                            Style::new()
                                        },
                                    ),
                            )
                            .style(Color::White)
                            .highlight_symbol(">>")
                            .highlight_spacing(
                                if app.state.focused_widget == FocusedWidget::BottomTable {
                                    HighlightSpacing::Always
                                } else {
                                    HighlightSpacing::Never
                                },
                            )
                            .row_highlight_style(
                                if app.state.focused_widget == FocusedWidget::BottomTable {
                                    Style::new().italic()
                                } else {
                                    Style::new()
                                },
                            )
                    } else {
                        let header = Row::new(["Index", "Title", "Type", "Url"]);
                        let rows: Vec<Row> = Vec::new();
                        let widths = [
                            Constraint::Percentage(5),
                            Constraint::Percentage(25),
                            Constraint::Percentage(20),
                            Constraint::Percentage(50),
                        ];
                        Table::new(rows, widths)
                            .header(header)
                            .block(
                                Block::bordered()
                                    .title_alignment(Alignment::Center)
                                    .title("No References")
                                    .style(
                                        if app.state.focused_widget == FocusedWidget::BottomTable {
                                            Style::new().bg(Color::DarkGray)
                                        } else {
                                            Style::new()
                                        },
                                    ),
                            )
                            .column_spacing(1)
                            .style(Color::White)
                    };

                    frame.render_stateful_widget(
                        widget_top,
                        layout[0],
                        &mut app.state.agol_content_widget_state,
                    );

                    frame.render_widget(widget_center, layout[1]);

                    frame.render_stateful_widget(
                        widget_bottom,
                        layout[2],
                        &mut app.state.reference_table_state,
                    );
                    // }
                }
            }
        }
    }
}
