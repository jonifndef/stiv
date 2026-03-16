use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Paragraph, Block, Borders}, Frame};
use crossterm::event::{self, Event};
use std::time::Duration;

#[derive(Default)]
pub struct App {
    exit: bool,
    path: String,
    msg: String,
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
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?
        }
        ratatui::restore();

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        //frame.render_widget("Hello world!", frame.area());

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
            Paragraph::new(self.msg.clone()).block(Block::new().borders(Borders::ALL)), cols_top[0]
        );

        frame.render_widget(
            Paragraph::new("Top right").block(Block::new().borders(Borders::ALL)), cols_top[1]
        );

        frame.render_widget(
            Paragraph::new("Bottom left").block(Block::new().borders(Borders::ALL)), cols_bot[0]
        );

        frame.render_widget(
            Paragraph::new("Bottom right").block(Block::new().borders(Borders::ALL)), cols_bot[1]
        );
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        //if matches!(event::read()?, Event::Key(_)) {
        //    self.exit = true;
        //}

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
