use clap::Parser;
use std::io::{self, Write};
use base64::{prelude::BASE64_STANDARD, Engine};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Png file to show
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();

    let control_data = b"f=100,t=f,a=T;";
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
