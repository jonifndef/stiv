use ratatui::{Frame};
use crossterm::event::{self, Event};

pub fn run(path: &str) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(display_image)?;
        if matches!(event::read()?, Event::Key(_)) {
            break;
        }
    }
    ratatui::restore();

    Ok(())
}

fn display_image(frame: &mut Frame) {
    frame.render_widget("Hello world!", frame.area());
}
