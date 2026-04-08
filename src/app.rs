use crossterm::event::{self, KeyCode};
use ratatui::{widgets::StatefulWidget, buffer::Buffer, prelude::Rect};
use std::{fs, io, path::{self, Path, PathBuf}};
use std::env;
use std::collections::HashMap;
use crate::{stiv_image::StivImage, ui};

//use std::time::Duration;
//use std::thread;

pub struct App {
    exit: bool,
    pub curr_mode: Mode,
    pub image_paths: Vec<String>,
    pub scroll_offset: u16,
    pub stiv_images: HashMap<String, StivImage>,
}

pub struct AppWidget;

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
            curr_mode: match &image_paths.len() {
                1 => Mode::SingleImage,
                _ => Mode::GalleryView
            },
            image_paths: image_paths,
            scroll_offset: 0,
            stiv_images: HashMap::new(),
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        while !self.exit {
            terminal.draw(|frame| frame.render_stateful_widget(AppWidget, frame.area(), self))?;
            self.handle_events()?;
            //thread::sleep(Duration::from_secs(5));
        }
        ratatui::restore();

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Char('j') => self.scroll_offset = self.scroll_offset.saturating_add(4),
                KeyCode::Char('k') => self.scroll_offset = self.scroll_offset.saturating_sub(4),
                _ => ()
            }
       }

        Ok(())
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
        ui::ui_draw(&area, buf, state);
    }
}
