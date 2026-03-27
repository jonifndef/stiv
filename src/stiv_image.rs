use imagesize::{size, ImageError};
use ratatui::{widgets::StatefulWidget, layout::Rect, buffer::Buffer};
use crate::{shm::ShmFile, win_info::WinInfo};
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{io::{self, Write}};
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
    pub cell_width_px: u16,
    pub cell_height_px: u16,
    size_rows: u16,
    pos_col: u16,
    pos_row: u16,
    id: u16,
    zoom_state: f32,
    dynamic_image: DynamicImage,
    resized_image: Option<DynamicImage>,
    shm_file: Option<ShmFile>,
}

impl StivImage {
    pub fn new(path: String, win_info: &WinInfo) -> anyhow::Result<StivImage> {
        let (img_width_px, img_height_px) = size(&path).map(|img_size| (img_size.width as u16, img_size.height as u16))?;
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
            shm_file: None,
        })
    }

    pub fn resize_to_fit(&mut self, area: &Rect) {
        let new_width = area.width * self.cell_width_px;
        let new_height = area.height * self.cell_height_px;

        if new_width > self.width_px ||
            new_height > self.height_px {
            return
        }

        self.resized_image = Some(self.dynamic_image.resize(new_width as u32, new_height as u32, image::imageops::FilterType::CatmullRom));
    }

    pub fn move_cursor(&mut self, area: &Rect) -> anyhow::Result<()> {
        let row = area.y + 1;
        let col = area.x + 1;
        let sequence = format!("\x1b[{row};{col}H").into_bytes();
        let mut stdout = io::stdout();

        stdout.write_all(&sequence)?;
        stdout.flush()?;

        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        let img = self.resized_image.clone().unwrap_or_else(|| self.dynamic_image.clone());
        let img_rgb = img.into_rgb8();
        let width = img_rgb.width();
        let height = img_rgb.height();
        let img_rgb_raw = img_rgb.into_raw();

        // ===========================//
        let mut shm = ShmFile::new(img_rgb_raw.len())?;
        shm.write_to_shm_file(&img_rgb_raw)?;

        let shm_path = shm.get_shm_path();
        //println!("{}", shm_path);
        let path_b64 = BASE64_STANDARD.encode(shm_path);
        let cmd = format!(
            "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
        );

        let mut stdout = io::stdout();
        stdout.write_all(cmd.as_bytes())?;
        stdout.flush()?;

        //self.shm_file = Some(shm);
        use std::time::Duration;
        use std::thread;
        thread::sleep(Duration::from_millis(5));
        // ===========================//

        //let encoded = BASE64_STANDARD.encode(img_rgb_raw);

        //let mut m = 1;
        //let chunk_itr = encoded.as_bytes().chunks(4096).with_position();
        //let mut stdout = io::stdout();
        //let mut out_buf: Vec<u8> = Vec::from([]);
        //for (pos, chunk) in chunk_itr {
        //    out_buf.extend(PREFIX);
        //    // TODO: what if image is only one chunk?
        //    if pos == Position::First {
        //        out_buf.extend(b"a=T,");
        //    }
        //    if pos == Position::Last {
        //        m = 0;
        //    }

        //    let control_data = format!("f=24,s={width},v={height},q=2,m={m};").into_bytes();
        //    out_buf.extend(control_data);
        //    out_buf.extend(chunk);
        //    out_buf.extend(SUFFIX);

        //    stdout.write_all(&out_buf)?;
        //    out_buf.clear();
        //}

        //stdout.flush()?;

        Ok(())
    }
}

pub struct StivImageWidget;

impl StatefulWidget for StivImageWidget {
    type State = StivImage;

    fn render(self, area: Rect, _buf: &mut Buffer, state: &mut StivImage) {
        // TODOS:
        //  - Only resize if it's needed! Compare area with self.resized_image, it might not
        // need to be resized even if we had a resize event from the terminal
        //  - Encode the kitty image buffer into the buf parameter
        //  - Maybe (!) set_skip() on cells that shouldn't be overwritten with spaces by Ratatui
        //  - Use faster resize crate, e.g. fast_image_resize, or even wgpu
        //  - In draw: Use shared memory for writing kitty buffer to terminal
        //  - In draw: Use tokio: tokio::io::stdout().write_all(&out_buf).await? to speed up
        //  gallery_view
        state.resize_to_fit(&area);

        if let Err(error) = state.move_cursor(&area) {
            println!("Error in state.move_cursor: {}", error)
        }

        if let Err(error) = state.draw() {
            println!("Error in state.draw: {}", error)
        }
    }
}
