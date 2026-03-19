use clap::Parser;
use stiv_image::StivImage;
use app::App;

mod app;
mod stiv_image;
mod win_info;
mod ui;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let mut app = App::new(args.file.clone());
    app.run()?;

    //let img = StivImage::new(args.file)?;
    //img.draw()?;

    Ok(())
}
