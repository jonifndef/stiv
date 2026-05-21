//use chrono::Duration;
use ratatui::{buffer::Buffer, layout::{Constraint, Direction, Layout, Rect,}, prelude::{StatefulWidget, Widget}, style::{Color, Style}, text::Line, widgets::{Block, BorderType, Borders, Scrollbar, ScrollbarOrientation, ScrollbarState}};
use rustix::path::Arg;
use crate::{app, win_info::WinInfo, App};
use crate::StivImage;
use crate::stiv_image::StivImageWidget;
use std::path::Path;
//use std::iter;

#[derive(Default)]
pub struct GalleryCursor {
    pub area: Rect,
    pub col: u16,
    pub row: u16,
}

#[derive(Default)]
pub struct Ui {
    pub scroll_offset: u16,
    pub current_selected_img_idx: usize,
    pub num_horizontal_grid_cells: usize,
    pub num_vertical_grid_cells: usize,
    pub grid_cell_width: usize,
    pub grid_cell_height: usize,
    pub visible_rows_under_selected_image: u16,
    pub gallery_cursor: GalleryCursor,
}

impl Ui {
    pub fn new() -> Self {
        let grid_cell_width = 30;
        let grid_cell_height = 12;

        Ui {
            scroll_offset: 0,
            current_selected_img_idx: 0,
            num_horizontal_grid_cells: 0,
            num_vertical_grid_cells: 0,
            grid_cell_width: grid_cell_width,
            grid_cell_height: grid_cell_height,
            visible_rows_under_selected_image: 0,
            gallery_cursor: GalleryCursor {
                area: Rect::new(0, 0, grid_cell_width as u16, grid_cell_height as u16),
                col: 0,
                row: 0,
            },
        }
    }

    // Set grid size based on terminal window cols,rows
    // 30x12 cells is a pretty good size to start with, per grid cell
    pub fn ui_draw(&mut self, rect: &Rect, buf: &mut Buffer, app: &mut App) {
        let win_info = match WinInfo::get_win_info() {
            Ok(win_info) => win_info,
            Err(error) => {
                println!("Error! {}", error);
                return;
            }
        };

        match app.curr_mode {
            app::Mode::SingleImage => {
                let img_path = app.image_paths[app.ui.current_selected_img_idx].clone();
                //if let Some(img) = app.stiv_images.get_mut(&img_path) {
                //    img.delete_from_terminal();
                //    std::thread::sleep(std::time::Duration::from_millis(750));

                //}
                self.draw_single_image(rect, buf, app, &win_info, &img_path);
            },
            app::Mode::GalleryView => self.draw_gallery_view(rect, buf, app, &win_info)
        }
    }

    fn draw_single_image(&self, area: &Rect, buffer: &mut Buffer, app: &mut App, win_info: &WinInfo, img_path: &String) {
        if !app.stiv_images.contains_key(img_path) {
            if let Ok(stiv_img) = StivImage::new(img_path.clone(), &win_info) {
                app.stiv_images.insert(img_path.clone(), stiv_img);
            } else {
                return
            }
        }

        //log::info!("For image: {}", img_path);
        //log::info!("draw_single_image: wininfo cols, rows: {}, {}", win_info.cols, win_info.rows);
        //log::info!("draw_single_image: wininfo cell_width_px, cell_height_px: {}, {}", win_info.cell_width_px, win_info.cell_height_px);
        //log::info!("draw_single_image: area.width, area.height: {}, {}", area.width, area.height);

        if let Some(stiv_img) = app.stiv_images.get_mut(img_path) {
            StivImageWidget.render(*area, buffer, stiv_img);
        }
    }

    fn draw_gallery_view(&mut self, area: &Rect, buffer: &mut Buffer, app: &mut App, win_info: &WinInfo) {
        let num_horizontal_grid_cells = (win_info.cols / self.grid_cell_width as u16) as u16;
        let num_vertical_grid_cells = (app.image_paths.len() as u16 + num_horizontal_grid_cells - 1) / num_horizontal_grid_cells as u16;

        self.num_horizontal_grid_cells = num_horizontal_grid_cells as usize;
        self.num_vertical_grid_cells = num_vertical_grid_cells as usize;

        let horizontal_constraints = vec![Constraint::Length(self.grid_cell_width as u16); num_horizontal_grid_cells as usize];
        let vertical_constraints = vec![Constraint::Length(self.grid_cell_height as u16); num_vertical_grid_cells as usize];

        let tot_content_height = num_vertical_grid_cells * self.grid_cell_height as u16;
        let tot_content_area = Rect::new(0, 0, win_info.cols, tot_content_height);
        let mut tot_content_buf = Buffer::empty(tot_content_area); // this buf will be passed as a mutref to each

        let scrollbar_needed = self.scroll_offset != 0 || tot_content_height > area.height;
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
        for (row_idx, row) in chunk_rows.into_iter().enumerate() {
            let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(horizontal_constraints.clone())
            .split(*row);

            for (col_idx, col) in cols.into_iter().enumerate() {
                let img_path = match app.image_paths.get(idx) {
                    Some(path) => {
                        path.clone()
                    },
                    None => {
                        break;
                    }
                };

                self.draw_single_image(col, &mut tot_content_buf, app, win_info, &img_path);

                if self.current_selected_img_idx == idx {
                    self.update_gallery_cursor(col, row_idx, col_idx);
                    self.draw_gallery_cursor(col, &img_path, &mut tot_content_buf);
                }

                idx += 1;
            }
        }

        let visible_content = tot_content_buf
            .content
            .into_iter()
            .skip((area.width * self.scroll_offset) as usize)
            .take(area.area() as usize);

        for (i, cell) in visible_content.enumerate() {
            let x = i as u16 % area.width;
            let y = i as u16 / area.width;
            buffer[(area.x + x, area.y + y)] = cell;
        }

        if scrollbar_needed {
            let area = area.intersection(buffer.area);
            let mut state = ScrollbarState::new(((num_vertical_grid_cells - 1) * self.grid_cell_height as u16) as usize)
                .position(self.scroll_offset as usize);
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(area, buffer, &mut state);
        }

        let current_selected_row = self.current_selected_img_idx / self.num_horizontal_grid_cells + 1;
        self.visible_rows_under_selected_image = area.height.saturating_sub(current_selected_row as u16 * self.grid_cell_height as u16);
        log::info!("visible_rows_under_selected_image: {}", self.visible_rows_under_selected_image);
        log::info!("area height: {}", area.height);
    }

    fn update_gallery_cursor(&mut self, area: &Rect, row_idx: usize, col_idx: usize) {
        self.gallery_cursor.area = *area;
        self.gallery_cursor.row = row_idx as u16;
        self.gallery_cursor.col = col_idx as u16;
    }

    fn draw_gallery_cursor(&self, area: &Rect, img_path: &String, buf: &mut Buffer) {
        let title = match Path::new(img_path).file_name() {
            Some(filename) => &String::from(filename.to_str().unwrap()),
            None => img_path
        };

        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(Color::White))
            .title_bottom(Line::from(title.as_str()).centered())
            .render(*area, buf);
    }
}
