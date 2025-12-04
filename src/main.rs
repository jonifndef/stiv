use clap::Parser;
use std::io::{self, Write};
use base64::{prelude::BASE64_STANDARD, Engine};
//use termion::raw::IntoRawMode;
//use std::io::{Write, stdout};
//use crossterm::{ terminal::{disable_raw_mode, enable_raw_mode, },};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    //let prefix = b"\033_G";
    let control_data = b"f=100;";
    //let delimiter = b";";
    let payload = std::fs::read(args.file)?;
    //let suffix = b"\033\\";

    let prefix = b"\x1b_G";
    let suffix = b"\x1b\\";

    let mut out_buf: Vec<u8> = vec![];
    out_buf.extend(prefix);
    out_buf.extend(control_data);
    //out_buf.extend(delimiter);
    out_buf.extend(BASE64_STANDARD.encode(payload).as_bytes());
    out_buf.extend(suffix);

    //enable_raw_mode()?;

    let mut stdout = io::stdout();
    stdout.write_all(&out_buf)?;
    stdout.flush()?;

    //disable_raw_mode()?;
    // From termion
    //let mut stdout = stdout().into_raw_mode()?;
    //write!(stdout, "{out_buf:?}")?;

    Ok(())
}
