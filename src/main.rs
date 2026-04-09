use clap::Parser;
use stiv_image::StivImage;
use app::App;
//use shm::ShmFile;
//use ratatui::layout::Rect;

mod app;
mod stiv_image;
mod win_info;
mod ui;
mod shm;
mod utils;
mod logging;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value_t = String::from(""))]
    file: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    logging::setup_logger().unwrap();

    let mut app = App::new(&args.file)?;
    app.run()?;

    Ok(())
}
