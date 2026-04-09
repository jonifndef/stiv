use imagesize::{size, ImageError};
use ratatui::{widgets::StatefulWidget, layout::Rect, buffer::Buffer};
use rustix::shm;
use crate::{shm::ShmFile, win_info::WinInfo, App};
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{io::{self, Write}};
use std::io::Cursor;
use image::{DynamicImage, ImageReader};
use itertools::{Itertools, Position};
use fast_image_resize as fir;
use fast_image_resize::images::Image as FirImage;

const PREFIX: &[u8] = b"\x1b_G";
const SUFFIX: &[u8] = b"\x1b\\";

pub struct StivImage {
    pub path: String,
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
    resized_image: Option<DynamicImage>,
    shm_file: Option<ShmFile>,
}

impl StivImage {
    pub fn new(path: String, win_info: &WinInfo) -> anyhow::Result<StivImage> {
        let (img_width_px, img_height_px) = size(&path).map(|img_size| (img_size.width as u16, img_size.height as u16))?;
        let img = image::open(path.as_str())?;

        let shm_file = if let Ok(shm_file) = ShmFile::new(img.to_rgb8().into_raw().len()) { Some(shm_file) } else { None };

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
            shm_file: shm_file,
        })
    }

    pub fn resize_to_fit(&mut self, area: &Rect) {
        let mut new_width = (area.width * self.cell_width_px) as u32;
        let mut new_height = (area.height * self.cell_height_px) as u32;

        if new_width > self.width_px as u32 &&
            new_height > self.height_px as u32 {
            return
        }

        (new_width, new_height) = self.adjust_for_aspect_ratio(new_width, new_height);

        let src_rgb = self.dynamic_image.to_rgb8();

        let src = fir::images::ImageRef::new(
            src_rgb.width(),
            src_rgb.height(),
            src_rgb.as_raw(),
            fir::PixelType::U8x3,
        ).unwrap();

        let mut dst = FirImage::new(new_width, new_height, fir::PixelType::U8x3);

        let mut resizer = fir::Resizer::new();

        resizer.resize(
            &src,
            &mut dst,
            &fir::ResizeOptions::new()
                .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3)),
        ).unwrap();

        let rgb_image = image::RgbImage::from_raw(
            new_width,
            new_height,
            dst.into_vec(),
        ).unwrap();

        self.resized_image = Some(DynamicImage::ImageRgb8(rgb_image));
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
        //let mut stdout = io::stdout();

        // ===========================//
        // Make this more obvious, somwthing like "if shm_available()"
        if let Some(shm_file) = &mut self.shm_file {
        //    self.draw_using_shm(&stdout, &img_rgb_raw)?;
        //} else {
        //    self.draw_using_byte_stream(&stdout, &img_rgb_raw)?;
            shm_file.resize_if_needed(img_rgb_raw.len())?;
            shm_file.write_to_shm_file(&img_rgb_raw)?;
            let path_b64 = BASE64_STANDARD.encode(shm_file.get_shm_path());
            let cmd = format!(
                "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
            );

            let mut stdout = io::stdout();
            stdout.write_all(cmd.as_bytes())?;
            stdout.flush()?;
        }


        //let cmd = format!(
        //    "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
        //);

        //let mut stdout = io::stdout();
        //stdout.write_all(cmd.as_bytes())?;
        //stdout.flush()?;

        //use std::time::Duration;
        //use std::thread;
        //thread::sleep(Duration::from_millis(5));
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

    //fn draw_using_shm(&self, stdout: &io::Stdout, img: &Container) -> anyhow::Result<()> {
    //    if let Some(shm_file) = self.shm_file {
    //        shm_file.resize_if_needed(img_rgb_raw.len())?;
    //        shm_file.write_to_shm_file(&img_rgb_raw)?;
    //    } else {
    //        return anyhow::Err;
    //    }

    //    Ok(())
    //}

    //fn draw_using_byte_stream(&self, stdout: &io::Stdout) -> anyhow::Result<()> {

    //    Ok(())
    //}

    fn adjust_for_aspect_ratio(&self, new_width: u32, new_height: u32) -> (u32, u32) {
        let ratio = self.dynamic_image.width() as f32 / self.dynamic_image.height() as f32;
        let test_width = new_height as f32 * ratio;

        if test_width > new_width as f32 {
            let height = new_width as f32 / ratio;
            // round the result?
            return (new_width, height as u32);

        }

        // round the result?
        return (test_width as u32, new_height);
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
            log::error!("Error in state.move_cursor: {}", error)
        }

        if let Err(error) = state.draw() {
            log::error!("Error in state.draw: {}", error)
        }
    }
}
