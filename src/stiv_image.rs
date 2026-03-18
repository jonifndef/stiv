use imagesize::{size, ImageError};
use ratatui::{widgets::StatefulWidget, layout::Rect, buffer::Buffer};
use crate::win_info::WinInfo;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{error, io::{self, Write}};
use std::io::Cursor;
use image::{DynamicImage, ImageReader};
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
    size_cols: u16,
    cell_width_px: u16,
    cell_height_px: u16,
    size_rows: u16,
    pos_col: u16,
    pos_row: u16,
    id: u16,
    zoom_state: f32,
    dynamic_image: DynamicImage,
    resized_image: Option<DynamicImage>
}

impl StivImage {
    pub fn new(path: String) -> Result<Self, anyhow::Error> {
        let (img_width_px, img_height_px) = size(&path).map(|img_size| (img_size.width as u16, img_size.height as u16))?;
        let win_info = WinInfo::get_win_info()?;
        let img = image::open(path.as_str())?;

        Ok(StivImage {
            path: path,
            width_px: img_width_px,
            height_px: img_height_px,
            size_cols: (img_width_px / win_info.cell_width_px),
            size_rows: (img_height_px / win_info.cell_height_px),
            cell_width_px: win_info.cell_width_px,
            cell_height_px: win_info.cell_height_px,
            pos_col: 0,
            pos_row: 0,
            id: 0,
            zoom_state: 1.0,
            dynamic_image: img,
            resized_image: None,
        })
    }

    pub fn resize_to_fit(&mut self, area: &Rect) {
        let new_width = (area.width * self.cell_width_px) as u32;
        let new_height = (area.height * self.cell_height_px) as u32;

        self.resized_image = Some(self.dynamic_image.clone().resize(new_width, new_height, image::imageops::FilterType::Nearest));
    }

    pub fn draw(&self, _pos_x: u16, _pos_y: u16) -> Result<(), anyhow::Error> {
        //let img_rgb = self.dynamic_image.into_rgb8();
        let img_rgb = self.resized_image.clone().into_rgb8();
        let width = img_rgb.width();
        let height = img_rgb.height();
        let img_rgb_raw = img_rgb.into_raw();
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

impl StatefulWidget for StivImage {
    type State = StivImage;

    fn render(self, area: Rect, _buf: &mut Buffer, state: &mut StivImage) {
        self.resize_to_fit(&area);
        self.draw(area.x, area.y);
    }
}
