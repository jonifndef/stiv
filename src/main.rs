use clap::Parser;
use stiv_image::StivImage;
use app::App;
use shm::ShmFile;

mod app;
mod stiv_image;
mod win_info;
mod ui;
mod shm;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let data = b"hello world";
    let mut shm = ShmFile::new(data.len())?;
    shm.write_to_shm_file(data);

    let mut app = App::new(&args.file)?;
    app.run()?;

//    let win_info = win_info::WinInfo::get_win_info()?;
//    let img = StivImage::new(args.file, &win_info)?;
//    img.draw()?;

    Ok(())
}
