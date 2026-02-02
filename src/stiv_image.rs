use imagesize::{size, ImageError};
use crate::win_info::WinInfo;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{error, io::{self, Write}};
use std::io::Cursor;
use image::ImageReader;
use itertools::{Itertools, Position};

// Some notes:
// If we are running in Kitty terminal, use only the cols OR rows argument in the control data, the
// other will scale dynamically, avoiding any unpleasant shrinkage artifacts and all that jazz.
// E.g.
//      let control_data = format!("f=100,t=f,a=T,c={cols};", cols=self.cols).into_bytes();
// However, if we are running on another kitty-supporting terminal, like wezterm or ghostty
// (unknown how ghostty does it), we need to provide both, as no dynamic scaling happens.
// E.g.
//      let control_data = format!("f=100,t=f,a=T,c={cols},r={rows};", cols=self.cols, rows=self.rows).into_bytes();

const PREFIX: &[u8] = b"\x1b_G";
const SUFFIX: &[u8] = b"\x1b\\";

pub struct StivImage {
    path: String,
    width_px: u16,
    height_px: u16,
    cols: u16,
    rows: u16,
    id: u16
}

impl StivImage {
    pub fn new(path: String) -> Result<Self, anyhow::Error> {
        let (img_width_px, img_height_px) = size(&path).map(|img_size| (img_size.width as u16, img_size.height as u16))?;

        let win_info = WinInfo::get_win_info()?;

        Ok(StivImage {
            path: path,
            width_px: img_width_px,
            height_px: img_width_px,
            cols: (img_width_px / win_info.cell_width),
            rows: (img_height_px / win_info.cell_height),
            id: 0
        })
    }

    pub fn draw(&self) -> Result<(), anyhow::Error> {
        let img_rbg = image::open(&self.path)?.into_rgb8();
        let width = img_rbg.width();
        let height = img_rbg.height();
        let img_rgb_raw = img_rbg.into_raw();
        let encoded = BASE64_STANDARD.encode(img_rgb_raw);

        let mut m = 1;
        let chunk_itr = encoded.as_bytes().chunks(4096).with_position();
        let mut stdout = io::stdout();
        let mut out_buf: Vec<u8> = Vec::from([]);
        for (pos, chunk) in chunk_itr {
            out_buf.extend(PREFIX);
            if pos == Position::First {
                out_buf.extend(b"a=T,");
            }
            if pos == Position::Last {
                m = 0;
            }

            let control_data = format!("f=24,s={width},v={height},m={m};").into_bytes();
            out_buf.extend(control_data);
            out_buf.extend(chunk);
            out_buf.extend(SUFFIX);

            stdout.write_all(&out_buf)?;
            out_buf.clear();
        }

        stdout.flush()?;

        Ok(())
    }
}
