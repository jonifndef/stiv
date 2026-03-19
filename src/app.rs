use crossterm::event::{self, Event};
use std::time::Duration;
use crate::ui;

pub struct App {
    exit: bool,
    pub path: String,
    pub msg: String,
}

impl App {
    pub fn new(path: String) -> Self {
        App {
            exit: false,
            path: path,
            msg: String::from("")
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        while !self.exit {
            //terminal.draw(|frame| self.draw(frame))?;
            terminal.draw(|frame| ui::ui_draw(frame, self))?;
            self.handle_events()?
        }
        ratatui::restore();

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(_) => self.exit = true,
                Event::Resize(cols, rows) => {
                    self.msg = format!("cols: {}, rows: {}", cols, rows);
                }

                _ => {}
            }
        }

        Ok(())
    }
}
