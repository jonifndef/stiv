use imagesize::{size, ImageError};
use crate::win_info::WinInfo;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{error, io::{self, Write}};

// Some notes:
// If we are running in Kitty terminal, use only the cols OR rows argument in the control data, the
// other will scale dynamically, avoiding any unpleasant shrinkage artifacts and all that jazz.
// E.g.
//      let control_data = format!("f=100,t=f,a=T,c={cols};", cols=self.cols).into_bytes();
// However, if we are running on another kitty-supporting terminal, like wezterm or ghostty
// (unknown how it works), we need to provide both, as no dynamic scaling happens.
// E.g.
//      let control_data = format!("f=100,t=f,a=T,c={cols},r={rows};", cols=self.cols, rows=self.rows).into_bytes();

const PREFIX: &'static [u8; 6] = b"\\x1b_G";
const SUFFIX: &'static [u8; 5] = b"\\x1b\\";

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
        let control_data = b"f=100,t=f,a=T;";
        let payload = self.path.as_bytes().to_vec();

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
}
