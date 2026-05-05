use clap::Parser;
use stiv_image::StivImage;
use app::App;
use ratatui::{buffer::Buffer, prelude::Rect};
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

    let mut app = App::new(&args.file)?;
    app.run()?;

    //let win_info = win_info::WinInfo::get_win_info()?;
    //let mut stiv_img = stiv_image::StivImage::new(String::from("assets/code.jpg"), &win_info)?;
    ////stiv_img.render_direct_transmission()?;
    //let area = Rect::new(0, 0, win_info.cols, win_info.rows);
    //stiv_img.upload_stream(&area)?;
    //let mut buf = Buffer::empty(area);
    //stiv_img.render_placeholders(area, &mut buf);

    Ok(())
}
