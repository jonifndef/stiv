use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Paragraph, Block, Borders}, Frame};
use crate::App;

pub fn ui_draw(frame: &mut Frame, app: &App) {
    let chunk_rows = Layout::default()
    .direction(Direction::Vertical)
    .constraints(vec![
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(frame.area());

    let cols_top = Layout::default()
    .direction(Direction::Horizontal)
    .constraints(vec![
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunk_rows[0]);

    let cols_bot = Layout::default()
    .direction(Direction::Horizontal)
    .constraints(vec![
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunk_rows[1]);

    frame.render_widget(
        Paragraph::new(app.msg.clone()).block(Block::new().borders(Borders::ALL)), cols_top[0]
    );

    frame.render_widget(
        Paragraph::new("Top right").block(Block::new().borders(Borders::ALL)), cols_top[1]
    );

    frame.render_widget(
        Paragraph::new("Bottom left").block(Block::new().borders(Borders::ALL)), cols_bot[0]
    );

    frame.render_widget(
        Paragraph::new("Bottom right").block(Block::new().borders(Borders::ALL)), cols_bot[1]
    );
}
