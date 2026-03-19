use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Paragraph, Block, Borders}, Frame};
use crate::App;
use crate::StivImage;
use crate::stiv_image::StivImageWidget;

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
        Paragraph::new(format!("Rect x,y, width,height: {},{} {},{}", cols_top[1].x, cols_top[1].y, cols_top[1].width, cols_top[1].height)).block(Block::new().borders(Borders::ALL)), cols_top[1]
    );

    if let Ok(mut stiv_img) = StivImage::new(app.path.clone()) {
        frame.render_stateful_widget(
            StivImageWidget, cols_bot[0], &mut stiv_img
        );
    }

    //frame.render_widget(
    //    Paragraph::new("Bottom left").block(Block::new().borders(Borders::ALL)), cols_bot[0]
    //);

    frame.render_widget(
        Paragraph::new("Bottom right").block(Block::new().borders(Borders::ALL)), cols_bot[1]
    );
}
