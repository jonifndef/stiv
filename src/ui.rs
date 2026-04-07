use ratatui::{prelude::{Widget, StatefulWidget}, layout::{Constraint, Direction, Layout, Rect,}, widgets::{Paragraph, Block, Borders, ScrollbarOrientation, ScrollbarState, Scrollbar}, Frame, buffer::Buffer};
use crate::{app, win_info::WinInfo, App};
use crate::StivImage;
use crate::stiv_image::StivImageWidget;
//use std::iter;

// Set grid size based on terminal window cols,rows
// 30x12 cells is a pretty good size to start with, per grid cell
pub fn ui_draw(rect: &Rect, buf: &mut Buffer, app: &mut App) {
    let win_info = match WinInfo::get_win_info() {
        Ok(win_info) => win_info,
        Err(error) => {
            println!("Error! {}", error);
            return;
        }
    };

    match app.curr_mode {
        app::Mode::SingleImage => draw_single_image(rect, buf, app, &win_info),
        app::Mode::GalleryView => draw_gallery_view(rect, buf, app, &win_info)
    }
}

fn draw_single_image(rect: &Rect, buffer: &mut Buffer, app: &mut App, win_info: &WinInfo) {
    if let Ok(mut stiv_img) = StivImage::new(app.image_paths[0].clone(), &win_info) {
        StivImageWidget.render(*rect, buffer, &mut stiv_img);
    }
}

fn draw_gallery_view(rect: &Rect, buffer: &mut Buffer, app: &App, win_info: &WinInfo) {
    // TODO: Dynamic, wrapping flex layout. Static grid element size, unless we zoom
    let grid_cell_width = 30;
    let grid_cell_height = 12;
    let tot_num_grid_cells = 20;

    let num_horizontal_grid_cells = (win_info.cols / grid_cell_width) as u16;
    let num_vertical_grid_cells = (tot_num_grid_cells + num_horizontal_grid_cells - 1) / num_horizontal_grid_cells as u16;

    let horizontal_constraints = vec![Constraint::Length(grid_cell_width); num_horizontal_grid_cells as usize];
    let vertical_constraints = vec![Constraint::Length(grid_cell_height); num_vertical_grid_cells as usize];

    let tot_content_height = num_vertical_grid_cells * grid_cell_height;
    let tot_content_area = Rect::new(0, 0, win_info.cols, tot_content_height);
    let mut tot_content_buf = Buffer::empty(tot_content_area); // this buf will be passed as a mutref to each

    let scrollbar_needed = app.scroll_offset != 0 || tot_content_height > rect.height;
    let content_area = if scrollbar_needed {
        Rect {
            width: tot_content_area.width - 1,
            ..tot_content_area
        }
    } else {
        tot_content_area
    };

    let mut grid_cells: Vec<Rect> = Vec::new();

    let chunk_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vertical_constraints)
        .split(content_area);

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
        if idx == app.image_paths.len() {
            break;
        }

        let mut img = match StivImage::new(app.image_paths[idx].clone(), win_info) {
            Ok(img) => img,
            Err(_err) => return
        };

        StivImageWidget.render(col, &mut tot_content_buf, &mut img);
        idx += 1;
    }

    let visible_content = tot_content_buf
        .content
        .into_iter()
        .skip((rect.width * app.scroll_offset) as usize) // it was "area" before
        .take(rect.area() as usize); // same here
    for (i, cell) in visible_content.enumerate() {
        let x = i as u16 % rect.width;
        let y = i as u16 / rect.width;
        buffer[(rect.x + x, rect.y + y)] = cell;
    }

    if scrollbar_needed {
        let area = rect.intersection(buffer.area);
        let mut state = ScrollbarState::new(((num_vertical_grid_cells - 1) * grid_cell_height) as usize)
            .position(app.scroll_offset as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(area, buffer, &mut state);
    }
}
