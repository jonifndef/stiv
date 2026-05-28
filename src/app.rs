use crossterm::event::{self, Event, KeyCode};
use ratatui::{widgets::StatefulWidget, buffer::Buffer, prelude::Rect};
use std::{fs, io, path::{self, Path, PathBuf}};
use std::env;
use std::collections::HashMap;
use crate::{stiv_event::StivEvent, stiv_image::StivImage, ui, detect_support};

pub struct App {
    exit: bool,
    pub current_mode: Mode,
    pub image_paths: Vec<String>,
    pub stiv_images: HashMap<String, StivImage>,
    pub ui: ui::Ui,
    pub current_event: StivEvent,
}

pub struct AppWidget;

#[derive(PartialEq)]
pub enum Mode {
    SingleImage,
    GalleryView
}

impl App {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let path_copy = if path.is_empty() {
            env::current_dir()?
        } else {
            Path::new(path).to_path_buf()
        };

        // Get support here, so we know if what kind of transmission we are to use (t= flag)
        // - Stream (d)
        // - Tmp file (t)
        // - Shm (s) - not working fully yet
        //
        // How to pass this information down to the stiv_image layer?

        let image_paths = get_image_paths(&path_copy)?;

        if image_paths.is_empty() {
            let mut err_msg = String::from("Path does not contain any images");
            if let Some(path_copy_str) = path_copy.to_str() {
                err_msg = format!("{}: {}", err_msg, path_copy_str);
            }

            return Err(anyhow::anyhow!(err_msg));
        }

        Ok(App {
            exit: false,
            current_mode: match &image_paths.len() {
                1 => Mode::SingleImage,
                _ => Mode::GalleryView
            },
            image_paths: image_paths,
            stiv_images: HashMap::new(),
            ui: ui::Ui::new(),
            current_event: StivEvent::None,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        while !self.exit {
            terminal.draw(|frame| frame.render_stateful_widget(AppWidget, frame.area(), self))?;

            self.handle_events()?;
        }

        self.delete_all_uploaded_images();

        ratatui::restore();

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Char('h') => self.handle_navigate_left(),
                KeyCode::Char('j') => self.handle_navigate_down(),
                KeyCode::Char('k') => self.handle_navigate_up(),
                KeyCode::Char('l') => self.handle_navigate_right(),
                KeyCode::Char('n') => self.handle_next(),
                KeyCode::Char('p') => self.handle_previous(),
                KeyCode::Char('+') => self.handle_zoom_in(),
                KeyCode::Enter => self.handle_toggle_mode(),
                _ => {}
            },
            Event::Resize(cols, rows) => {
                self.handle_resize(cols, rows);
            },
            _ => {}
        }

        Ok(())
    }

    fn handle_navigate_left(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                log::info!("Panning left in SingleImage mode");
            },
            Mode::GalleryView => {
                if self.ui.current_selected_img_idx % self.ui.num_horizontal_grid_cells == 0 {
                    return
                }

                self.ui.current_selected_img_idx = self.ui.current_selected_img_idx.saturating_sub(1);
            }
        }
    }

    fn handle_navigate_down(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                log::info!("Panning down in SingleImage mode");
            },
            Mode::GalleryView => {
                if self.ui.current_selected_img_idx >= self.stiv_images.len() - self.ui.num_horizontal_grid_cells {
                    return
                }

                self.ui.current_selected_img_idx = self.ui.current_selected_img_idx + self.ui.num_horizontal_grid_cells;

                if self.ui.visible_rows_under_selected_image < self.ui.grid_cell_height as u16 {
                    self.ui.scroll_offset = self.ui.scroll_offset.saturating_add(self.ui.grid_cell_height as u16 - self.ui.visible_rows_under_selected_image);
                }
            }
        }
    }

    fn handle_navigate_up(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                log::info!("Panning up in SingleImage mode");
            },
            Mode::GalleryView => {
                if self.ui.current_selected_img_idx < self.ui.num_horizontal_grid_cells {
                    return
                }

                self.ui.current_selected_img_idx = self.ui.current_selected_img_idx - self.ui.num_horizontal_grid_cells;

                let num_rows_above_cursor = self.ui.gallery_cursor.area.y - self.ui.scroll_offset;
                if num_rows_above_cursor < self.ui.gallery_cursor.area.height {
                    self.ui.scroll_offset = self.ui.scroll_offset.saturating_sub(self.ui.grid_cell_height as u16 - num_rows_above_cursor);
                }
            }
        }
    }

    fn handle_navigate_right(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                log::info!("Panning right in SingleImage mode");
            },
            Mode::GalleryView => {
                if (self.ui.current_selected_img_idx % self.ui.num_horizontal_grid_cells) == (self.ui.num_horizontal_grid_cells - 1) {
                    return
                }

                self.ui.current_selected_img_idx = self.ui.current_selected_img_idx.saturating_add(1);
            }
        }
    }

    fn handle_next(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                if self.ui.current_selected_img_idx < self.image_paths.len() {
                    self.ui.current_selected_img_idx = self.ui.current_selected_img_idx.saturating_add(1)
                }
            },
            _ => ()
        }
    }

    fn handle_previous(&mut self) {
        match self.current_mode {
            Mode::SingleImage => {
                self.ui.current_selected_img_idx = self.ui.current_selected_img_idx.saturating_sub(1)
            },
            _ => ()
        }
    }

    fn handle_zoom_in(&mut self) {
        // It is better to set a field in the StivImageWidget struct that gets created per-frame.
        // This information does not need to be
        // a) global, it only affects a single stiv_image most of the time, and
        // b) persistent, even as it is now, the event state is persistent in App, but it needn't
        // be, it's per-frame data
        // This probably goes for all StivEvent:s
        match self.current_mode {
            Mode::SingleImage => {
                let current_img_path = &self.image_paths[self.ui.current_selected_img_idx];
                let current_stiv_img = self.stiv_images.get_mut(current_img_path).unwrap();

                current_stiv_img.zoom_state = current_stiv_img.zoom_state + 0.25;
                self.current_event = StivEvent::ZoomIn;
            },
            Mode::GalleryView => {
                log::info!("Zooming in in gallery view!");
            }
        }
    }

    fn handle_toggle_mode(&mut self) {
        self.current_mode = if self.current_mode == Mode::GalleryView { Mode::SingleImage } else { Mode::GalleryView };
        self.current_event = StivEvent::ToggleMode;
    }

    fn handle_resize(&mut self, _cols: u16, _rows: u16) {
        self.current_event = StivEvent::TermResize;
    }

    fn delete_all_uploaded_images(&mut self) {
        for path in &self.image_paths {
            if let Some(img) = self.stiv_images.get_mut(path) {
                img.delete_from_terminal();
            }
        }
    }
}

fn get_image_paths(path: &PathBuf) -> io::Result<Vec<String>> {
    let mut image_paths = vec![];

    if is_image(path) {
        if let Some(path_str) = path.to_str() {
            image_paths.push(String::from(path_str));
        }

        return Ok(image_paths)
    }

    let dir_entries = fs::read_dir(path)?;

    for entry in dir_entries {
        let full_path = entry?.path();
        if full_path.is_dir() {
            continue;
        }

        if !is_image(&full_path) {
            continue;
        }

        if let Some(full_path_str) = full_path.to_str() {
            image_paths.push(String::from(full_path_str));
        }
    }

    Ok(image_paths)
}

fn is_image(path: &path::PathBuf) -> bool {
    // Something to dwell on: Maybe use the "infer" create to read the file contents and infer the
    // file type from it. That feels a bit costly, though, as that would lead to two file reads per
    // file. The following implementation is an accepteable compromise for the time being.
    // Antoher thing: move the logic to discover file extenstions to other place, and use that in
    // stiv_image.rs to find out which "transmission protocol" to use: shared mem, tmp file, chunks
    // to stdout, or... PNG!
    return match path.extension().and_then(|ext| ext.to_str()) {
        Some("jpg") => true,
        Some("jpeg") => true,
        Some("png") => true,
        _ => false
    }
}

impl StatefulWidget for AppWidget {
    type State = App;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut App) {
        let mut ui = std::mem::take(&mut state.ui);
        ui.ui_draw(&area, buf, state);
        state.ui = ui;
    }
}
