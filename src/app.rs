use crossterm::event::{self, Event};
use std::time::Duration;
use std::{fs, io, path};
use crate::ui;
//use std::thread;

pub struct App {
    exit: bool,
    pub msg: String,
    curr_mode: Mode,
    pub image_paths: Vec<String>,
}

enum Mode {
    SingleImage,
    GalleryView
}

impl App {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let image_paths = get_image_paths(path)?;

        Ok(App {
            exit: false,
            msg: String::from(""),
            curr_mode: match &image_paths.len() {
                0 => {
                    Mode::SingleImage
                },
                1 => Mode::SingleImage,
                _ => Mode::GalleryView
            },
            image_paths: image_paths,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        while !self.exit {
            //terminal.draw(|frame| self.draw(frame))?;
            terminal.draw(|frame| ui::ui_draw(frame, self))?;
            self.handle_events()?;
            //thread::sleep(Duration::from_secs(5));
        }
        ratatui::restore();

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
       match event::read()? {
           Event::Key(_) => self.exit = true,
           Event::Resize(cols, rows) => {
               self.msg = format!("cols: {}, rows: {}", cols, rows);
           }

           _ => {}
       }

        Ok(())
    }

}

fn get_image_paths(path: &str) -> io::Result<Vec<String>> {
    let dir_entries = fs::read_dir(path)?;
    let mut image_paths = vec![];

    for entry in dir_entries {
        let full_path = entry?.path();
        if full_path.is_dir() {
            continue;
        } else {
            if is_image(&full_path) {
                if let Some(full_path_str) = full_path.to_str() {
                    image_paths.push(String::from(full_path_str));
                }
            }
        }
    }

    Ok(image_paths)
}

fn is_image(path: &path::PathBuf) -> bool {
    // Something to dwell on: Maybe use the "infer" create to read the file contents and infer the
    // file type from it. That feels a bit costly, though, as that would lead to two file reads per
    // file. The following implementation is an accepteable compromise for the time being.
    if let Some(extension) = path.extension() {
        match extension {
            _ => true
        }
    } else {
        false
    }
}
