use clap::Parser;
use std::io::{self, Write};
use base64::{prelude::BASE64_STANDARD, Engine};
use crossterm::terminal::{self, window_size};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

struct Rect {
    width_px: u16,
    height_px: u16
}

fn get_cell_size() -> Result<Rect, std::io::Error> {
    let window_size = terminal::window_size()?;

    let cell_size = Rect {
        width_px: (window_size.width / window_size.columns),
        height_px: (window_size.height / window_size.rows)
    };

    Ok(cell_size)
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let (cols, rows) = terminal::size().map(|(c,r)|(c/2, r/2))?;

    println!("cols: {}, rows: {}", cols, rows);

    let window_size = terminal::window_size()?;
    let img_width = window_size.width / 2;

    //let control_data = b"f=100,t=f,a=T,s=33,v=11;";
    //let control_data = format!("f=100,t=f,a=T,c={cols},r={rows};").into_bytes();
    let control_data = format!("f=100,t=f,a=T,c={cols};").into_bytes();
    //let control_data = format!("f=100,t=f,a=T,w={img_width};").into_bytes();
    //let payload = std::fs::read(args.file)?;
    let payload = args.file.as_bytes().to_vec();

    let prefix = b"\x1b_G";
    let suffix = b"\x1b\\";

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
