use clap::Parser;
use std::{fs::File, io::Read};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() {
    let args = Args::parse();
    if let Ok(img_data) = std::fs::read(args.file) {
        println!("{img_data:?}");
    }

    println!("stiv - simple terminal image viewer");
}
