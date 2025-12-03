use clap::Parser;
use std::io::{self, Write};
use base64::{prelude::BASE64_STANDARD};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let prefix = b"\033_G";
    let control_data = b"f=100;";
    let payload = std::fs::read(args.file)?;
    let appendix = b"\033\\";

    let mut out_buf: Vec<i8> = vec![];
    out_buf.extend_from_slice(prefix);
    out_buf.extend_from_slice(control_data);
    out_buf.extend_from_slice(BASE64_STANDARD.encode(payload).as_bytes());
    out_buf.extend_from_slice(appendix);

    let mut stdout = io::stdout();
    stdout.write_all(&img_data)?;
    stdout.flush()?;

    Ok(())
}
