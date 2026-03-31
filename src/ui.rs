use ratatui::{layout::{Constraint, Direction, Layout, Rect}, widgets::{Paragraph, Block, Borders}, Frame};
use crate::{app, App};
use crate::StivImage;
use crate::stiv_image::StivImageWidget;
//use std::iter;

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

    let num_vertical_grid_cells = (app.win_info.rows / 12) as u16;
    let num_horizontal_grid_cells = (app.win_info.cols / 30) as u16;
    let perc_v = 100 / num_vertical_grid_cells as u16;
    let perc_h = 100 / num_horizontal_grid_cells as u16;

    let vertical_constraints = vec![Constraint::Percentage(perc_v); num_vertical_grid_cells as usize];
    let horizontal_constraints = vec![Constraint::Percentage(perc_h); num_horizontal_grid_cells as usize];

    let mut grid_cells: Vec<Rect> = Vec::new();

    let chunk_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vertical_constraints)
        .split(frame.area());

    for row in chunk_rows.into_iter() {
        let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(horizontal_constraints.clone())
        .split(*row);

        for col in cols.into_iter() {
            grid_cells.push(*col);
        }
    }

    let mut idx = 0;
    for col in grid_cells.into_iter() {
        let msg = format!("Ollebolle: {}", idx);
        idx += 1;
        frame.render_widget(Paragraph::new(msg).block(Block::new().borders(Borders::ALL)), col);
    }

    //frame.render_widget(
    //    Paragraph::new(app.msg.clone()).block(Block::new().borders(Borders::ALL)), cols_top[0]
    //);

    //if let Ok(mut stiv_img) = StivImage::new(app.image_paths[0].clone(), &app.win_info) {
    //    frame.render_stateful_widget(
    //        StivImageWidget, cols_bot[0], &mut stiv_img
    //    );

    //    frame.render_widget(
    //        Paragraph::new(format!("Rect x,y, width,height: {},{} {},{}, cell_wdith, cell_height: {},{}", cols_top[1].x, cols_top[1].y, cols_top[1].width, cols_top[1].height, stiv_img.cell_width_px, stiv_img.cell_height_px)).block(Block::new().borders(Borders::ALL)), cols_top[1]
    //    );
    //}

    //if let Ok(mut stiv_img) = StivImage::new(app.image_paths[1].clone(), &app.win_info) {
    //    frame.render_stateful_widget(
    //        StivImageWidget, cols_bot[1], &mut stiv_img
    //    );
    //}
}

fn get_num_horizontal_grid_cells(window_cols: u16) -> u16 {
    // start by dividing by a middle-ground width, something like 30, save the truncated int and check if the reminder is
    // under or above 0.5.
    // If it's under 0.5, increase width from 30 to 31, if it's still above or equal
    // to the old int, step up to 32, keep checking. If it's under, use the previous width.
    // If it's over 0.5, decrease the width from 30 to 29, if it's still under or equal to the old
    // int, step down to 28, keep checking. If it's above, use the previous width
    let with_decimal_points = window_cols / 30;
    let truncated_int = with_decimal_points as i16;
    if (truncated_int as f32 + 0.5) as i16 > truncated_int {

    }

    3
    // we need to return the width itself too! It is needed in the grid cell constraints!
}
