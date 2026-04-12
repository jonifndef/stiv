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

    let img_path = app.image_paths[0].clone();

    match app.curr_mode {
        app::Mode::SingleImage => draw_single_image(rect, buf, app, &win_info, img_path),
        app::Mode::GalleryView => draw_gallery_view(rect, buf, app, &win_info)
    }
}

fn draw_single_image(area: &Rect, buffer: &mut Buffer, app: &mut App, win_info: &WinInfo, img_path: String) {
    if !app.stiv_images.contains_key(&img_path) {
        if let Ok(stiv_img) = StivImage::new(img_path.clone(), &win_info) {
            app.stiv_images.insert(img_path.clone(), stiv_img);
        } else {
            return
        }
    }

    if let Some(stiv_img) = app.stiv_images.get_mut(&img_path) {
        StivImageWidget.render(*area, buffer, stiv_img);
    }
}

fn draw_gallery_view(area: &Rect, buffer: &mut Buffer, app: &mut App, win_info: &WinInfo) {
    let grid_cell_width = 30;
    let grid_cell_height = 12;

    let num_horizontal_grid_cells = (win_info.cols / grid_cell_width) as u16;
    let num_vertical_grid_cells = (app.image_paths.len() as u16 + num_horizontal_grid_cells - 1) / num_horizontal_grid_cells as u16;

    let horizontal_constraints = vec![Constraint::Length(grid_cell_width); num_horizontal_grid_cells as usize];
    let vertical_constraints = vec![Constraint::Length(grid_cell_height); num_vertical_grid_cells as usize];

    let tot_content_height = num_vertical_grid_cells * grid_cell_height;
    let tot_content_area = Rect::new(0, 0, win_info.cols, tot_content_height);
    let mut tot_content_buf = Buffer::empty(tot_content_area); // this buf will be passed as a mutref to each

    let scrollbar_needed = app.scroll_offset != 0 || tot_content_height > area.height;
    let content_area = if scrollbar_needed {
        Rect {
            width: tot_content_area.width - 1,
            ..tot_content_area
        }
    } else {
        tot_content_area
    };

    let chunk_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vertical_constraints)
        .split(content_area);

    let mut idx = 0;
    for row in chunk_rows.into_iter() {
        let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(horizontal_constraints.clone())
        .split(*row);

        for col in cols.into_iter() {
            let img_path = match app.image_paths.get(idx) {
                Some(path) => {
                    path.clone()
                },
                None => {
                    break;
                }
            };

            //let msg = format!("Ollebolle: {}", idx);
            //Paragraph::new(msg).block(Block::new().borders(Borders::ALL)).render(*col, &mut tot_content_buf);
            draw_single_image(col, &mut tot_content_buf, app, win_info, img_path);
            idx += 1;
        }
    }

    let visible_content = tot_content_buf
        .content
        .into_iter()
        .skip((area.width * app.scroll_offset) as usize)
        .take(area.area() as usize);
    for (i, cell) in visible_content.enumerate() {
        let x = i as u16 % area.width;
        let y = i as u16 / area.width;
        buffer[(area.x + x, area.y + y)] = cell;
    }

    if scrollbar_needed {
        let area = area.intersection(buffer.area);
        let mut state = ScrollbarState::new(((num_vertical_grid_cells - 1) * grid_cell_height) as usize)
            .position(app.scroll_offset as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(area, buffer, &mut state);
    }
}
