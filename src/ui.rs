use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Paragraph, Block, Borders}, Frame};
use crate::{app, App};
use crate::StivImage;
use crate::stiv_image::StivImageWidget;

// Set grid size based on terminal window cols,rows
// 30x12 cells is a pretty good size to start with, per grid cell
pub fn ui_draw(frame: &mut Frame, app: &App) {
    match app.curr_mode {
        app::Mode::SingleImage => draw_single_image(frame, app),
        app::Mode::GalleryView => draw_gallery_view(frame, app)
    }
}

fn draw_single_image(frame: &mut Frame, app: &App) {
    // Set grid size (w,h) based on num of cols,rows in window
    if let Ok(mut stiv_img) = StivImage::new(app.image_paths[0].clone(), &app.win_info) {
        frame.render_stateful_widget(StivImageWidget, frame.area(), &mut stiv_img);
    }
}

fn draw_gallery_view(frame: &mut Frame, app: &App) {
// TODO: Dynamic, wrapping flex layout. Static grid element size, unless we zoom
//
//    let num_horizontal_grid_cells = (app.win_info.cols / 30) as i16;
//    let num_vertical_grid_cells = (app.win_info.rows / 12) as i16;
//
//    let chunk_rows = Layout::default()
//        .direction(Direction::Vertical)
//        .constraints(constraints)
//}
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

    if let Ok(mut stiv_img) = StivImage::new(app.image_paths[0].clone(), &app.win_info) {
        frame.render_stateful_widget(
            StivImageWidget, cols_bot[0], &mut stiv_img
        );

        frame.render_widget(
            Paragraph::new(format!("Rect x,y, width,height: {},{} {},{}, cell_wdith, cell_height: {},{}", cols_top[1].x, cols_top[1].y, cols_top[1].width, cols_top[1].height, stiv_img.cell_width_px, stiv_img.cell_height_px)).block(Block::new().borders(Borders::ALL)), cols_top[1]
        );
    }

    if let Ok(mut stiv_img) = StivImage::new(app.image_paths[1].clone(), &app.win_info) {
        frame.render_stateful_widget(
            StivImageWidget, cols_bot[1], &mut stiv_img
        );
    }
}
