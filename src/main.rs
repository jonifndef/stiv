use clap::Parser;
use std::{error, io::{self, Write}};
use base64::{prelude::BASE64_STANDARD, Engine};
use crossterm::terminal::{self, window_size, WindowSize};
use imagesize::{size, ImageError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

struct Rect {
    width: u16,
    height: u16
}

fn get_term_cell_size_px(window_size: &WindowSize) -> Rect {
    let cell_size = Rect {
        width: (window_size.width / window_size.columns),
        height: (window_size.height / window_size.rows)
    };

    cell_size
}

fn get_image_rows_and_cols(image_path: &str, window_size: &WindowSize, scaling_percent: u8, cell_size: &Rect) -> Result<Rect, ImageError> {
    // Get the width and height of image
    let (img_width_px, img_height_px) = size(image_path).map(|img_size| (img_size.width, img_size.height))?;

    let mut scaled_img_width_px: u16 = 0;
    let mut scaled_img_height_px: u16 = 0;

    // Check if any side of the image is longer than "scale_in_percent".
    if ((img_width_px as u16) > (window_size.width * ((scaling_percent as u16) / 100))) || ((img_height_px as u16) > (window_size.height * ((scaling_percent as u16) / 100))) {
        // If so, multiply both sides with percent, then divide the result with cell_size
        scaled_img_width_px = (img_width_px as u16) * (scaling_percent as u16);
        scaled_img_height_px = (img_height_px as u16) * (scaling_percent as u16);
        // Bonus: Do we want to be able to specify a dimension of the image to scale?
        // As in: "scale img height by 70%" or "scale image width by 35%"
        // This block does not support that scenario.
    }

    Ok(Rect {
        width: scaled_img_width_px / cell_size.width,
        height: scaled_img_height_px / cell_size.height
    })
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let (cols, rows) = terminal::size().map(|(c,r)|(c/2, r/2))?;

    println!("cols: {}, rows: {}", cols, rows);

    let window_size = terminal::window_size()?;
    let img_width = window_size.width / 2;

    let control_data = b"f=100,t=f,a=T;";
    //let control_data = format!("f=100,t=f,a=T,c={cols},r={rows};").into_bytes();
    //let control_data = format!("f=100,t=f,a=T,c={cols};").into_bytes();
    //let control_data = format!("f=100,t=f,a=T,w={img_width};").into_bytes();
    //let payload = std::fs::read(args.file)?;
    let payload = args.file.as_bytes().to_vec();

    let prefix = b"\\x1b_G";
    let suffix = b"\\x1b\\";

    let mut out_buf: Vec<u8> = vec![];
    out_buf.extend(prefix);
    out_buf.extend(control_data);
    out_buf.extend(BASE64_STANDARD.encode(payload).as_bytes());
    out_buf.extend(suffix);

    let mut stdout = io::stdout();
    stdout.write_all(&out_buf)?;
    stdout.flush()?;

    Ok(())
}
