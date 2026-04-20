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
mod kitty_diacritics;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value_t = String::from(""))]
    file: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    logging::setup_logger().unwrap();

    //let mut app = App::new(&args.file)?;
    //app.run()?;

    let win_info = win_info::WinInfo::get_win_info()?;
    let mut stiv_img = stiv_image::StivImage::new(String::from("assets/code.jpg"), &win_info)?;
    stiv_img.render_direct_transmission()?;

    Ok(())
}
