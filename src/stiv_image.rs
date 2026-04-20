use imagesize::{size};
use ratatui::{widgets::StatefulWidget, layout::Rect, buffer::Buffer};
use crate::{shm::ShmFile, win_info::WinInfo, kitty_diacritics};
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{io::{self, Write}};
use image::{DynamicImage};
use fast_image_resize as fir;
use fast_image_resize::images::Image as FirImage;
use std::sync::atomic::{AtomicU32, Ordering};
use ratatui::style::Color;
use itertools::{Itertools, Position};

static NEXT_KITTY_ID: AtomicU32 = AtomicU32::new(1);

const PREFIX: &[u8] = b"\x1b_G";
const SUFFIX: &[u8] = b"\x1b\\";
const PLACEHOLDER: &str = "\u{10EEEE}";

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
    id: u32,
    pub uploaded: bool,
    pub last_area: Option<Rect>,
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
            id: NEXT_KITTY_ID.fetch_add(1, Ordering::Relaxed),
            uploaded: false,
            last_area: None,
            zoom_state: 1.0,
            dynamic_image: img,
            resized_image: None,
            shm_file: shm_file,
        })
    }

    pub fn resize_to_fit(&mut self, area: &Rect) -> Rect {
        log::info!("resize_to_fit called!");
        let mut new_width = (area.width * self.cell_width_px) as u32;
        let mut new_height = (area.height * self.cell_height_px) as u32;

        if new_width > self.width_px as u32 &&
            new_height > self.height_px as u32 {
            return Rect::new(area.x, area.y, area.width, area.height)
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
        self.uploaded = false;

        return Rect::new(area.x, area.y, new_width as u16 / self.cell_width_px, new_height as u16 / self.cell_height_px);
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

    pub fn upload_stream(&mut self, area: &Rect) -> anyhow::Result<()> {
        let img = self.resized_image.as_ref()
            .unwrap_or(&self.dynamic_image);
        let img_rgb = img.to_rgb8();
        let width  = img_rgb.width();
        let height = img_rgb.height();
        let raw    = img_rgb.as_raw();

        let encoded = BASE64_STANDARD.encode(raw);
        let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(4096).collect();
        let last_idx = chunks.len().saturating_sub(1);

        let id   = self.id;
        let cols = area.width;
        let rows = area.height;

        let mut stdout = io::stdout();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_first = i == 0;
            let is_last  = i == last_idx;
            let m = if is_last { 0 } else { 1 };

            let control = if is_first {
                // All metadata on the first chunk only
                format!("a=T,f=24,t=d,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2,m={m}")
            } else {
                format!("m={m},i={id},q=2")
            };

            let mut out = Vec::new();
            out.extend_from_slice(b"\x1b_G");
            out.extend_from_slice(control.as_bytes());
            out.push(b';');
            out.extend_from_slice(chunk);
            out.extend_from_slice(b"\x1b\\");
            stdout.write_all(&out)?;
        }

        stdout.flush()?;
        self.uploaded = true;
        self.last_area = Some(*area);
        Ok(())
    }

    pub fn upload_shm(&mut self, area: &Rect) -> anyhow::Result<()> {
        let img = self.resized_image.clone().unwrap_or_else(|| self.dynamic_image.clone());
        let img_rgb = img.into_rgb8();
        let width = img_rgb.width();
        let height = img_rgb.height();
        let img_rgb_raw = img_rgb.into_raw();

        // ===========================//
        // Make this more obvious, somwthing like "if shm_available()"
        if let Some(shm_file) = &mut self.shm_file {
        //    self.draw_using_shm(&stdout, &img_rgb_raw)?;
        //} else {
        //    self.draw_using_byte_stream(&stdout, &img_rgb_raw)?;

            shm_file.resize_if_needed(img_rgb_raw.len())?;
            shm_file.write_to_shm_file(&img_rgb_raw)?;

            let path_b64 = BASE64_STANDARD.encode(shm_file.get_shm_path());
            let id = self.id;
            let rows = area.height;
            let cols = area.width;

            //let cmd = format!(
            //    "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
            //);

            let cmd = format!(
                "\x1b_Ga=T,f=24,t=s,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2;{path_b64}\x1b\\",
            );

            let mut stdout = io::stdout();
            stdout.write_all(cmd.as_bytes())?;
            stdout.flush()?;
            std::thread::sleep(std::time::Duration::from_millis(750));

            self.uploaded = true;
            self.last_area = Some(*area);
        }

        Ok(())
    }

    pub fn render_placeholders(&self, area: Rect, buf: &mut Buffer) {
        let id = self.id;

        // Encode image ID as an RGB foreground color:
        // red   = (id >> 16) & 0xff
        // green = (id >>  8) & 0xff
        // blue  =  id        & 0xff
        let r = ((id >> 16) & 0xff) as u8;
        let g = ((id >>  8) & 0xff) as u8;
        let b = (id         & 0xff) as u8;
        let color = Color::Rgb(r, g, b);

        for row in 0..area.height {
            for col in 0..area.width {
                let row_diacritic = kitty_diacritics::diacritic_for_index(row as u32);
                let col_diacritic = kitty_diacritics::diacritic_for_index(col as u32);

                let placeholder = format!(
                    "{PLACEHOLDER}{row_diacritic}{col_diacritic}"
                );

                let cell = buf.cell_mut((area.x + col, area.y + row));
                if let Some(cell) = cell {
                    cell.set_symbol(&placeholder)
                        .set_fg(color);
                }
            }
        }
    }

    pub fn render_direct_transmission(&mut self) -> anyhow::Result<()> {
        let img = self.resized_image.clone().unwrap_or_else(|| self.dynamic_image.clone());
        let img_rgb = img.into_rgb8();
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
            // TODO: what if image is only one chunk?
            if pos == Position::First {
                out_buf.extend(b"a=T,");
            }
            if pos == Position::Last {
                m = 0;
            }

            let control_data = format!("f=24,s={width},v={height},q=2,m={m};").into_bytes();
            out_buf.extend(control_data);
            out_buf.extend(chunk);
            out_buf.extend(SUFFIX);

            stdout.write_all(&out_buf)?;
            out_buf.clear();
        }

        stdout.flush()?;

        Ok(())
    }

    pub fn draw(&mut self, area: &Rect, buf: &mut Buffer) -> anyhow::Result<()> {
        let img = self.resized_image.clone().unwrap_or_else(|| self.dynamic_image.clone());
        let img_rgb = img.into_rgb8();
        let width = img_rgb.width();
        let height = img_rgb.height();
        let img_rgb_raw = img_rgb.into_raw();

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

            buf.cell_mut((area.x, area.y))
               .unwrap()
               .set_symbol(&cmd);

            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    if x == area.x && y == area.y {
                        continue; // the first cell
                    }
                    buf.cell_mut((x, y))
                        .unwrap()
                        .set_skip(true);
                }
            }
        }

        //let cmd = format!(
        //    "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
        //);

        //let mut stdout = io::stdout();
        //stdout.write_all(cmd.as_bytes())?;
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

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut StivImage) {
        let new_area = state.resize_to_fit(&area);
        //log::info!("area x,y,w,h: {},{},{},{}", area.x, area.y, area.width, area.height);
        //log::info!("new_area x,y,w,h: {},{},{},{}", new_area.x, new_area.y, new_area.width, new_area.height);

        //let mut stdout = io::stdout();
        //stdout.write_all(b"\x1b[s").unwrap();
        //if let Err(error) = state.move_cursor(&area) {
        //    log::error!("Error in state.move_cursor: {}", error)
        //}

        let needs_upload = !state.uploaded
            || state.last_area != Some(new_area);

        if needs_upload {
            if let Err(e) = state.upload_shm(&new_area) {
                log::error!("upload error: {e}");
                return;
            }
        }

        state.render_placeholders(new_area, buf);

        //if let Err(error) = state.draw(&area, buf) {
        //    log::error!("Error in state.draw: {}", error)
        //}
        //stdout.write_all(b"\x1b[u").unwrap();
    }
}
