use imagesize::{size, ImageError};
use crate::win_info::WinInfo;

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

    pub fn draw() -> Result<(), anyhow::Error> {
        Ok(())
    }
}
