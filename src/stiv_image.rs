use imagesize::{size};
use ratatui::{widgets::StatefulWidget, layout::Rect, buffer::Buffer};
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{fmt::Write, io::{self, Write as stdoutWrite}, thread, time::Duration};
use image::{DynamicImage};
use fast_image_resize as fir;
use fast_image_resize::images::Image as FirImage;
use std::sync::atomic::{AtomicU32, Ordering};
use ratatui::style::Color;
use itertools::{Itertools, Position};
use crate::{kitty_diacritics, shm::ShmFile, stiv_event::StivEvent, win_info::WinInfo};

static NEXT_KITTY_ID: AtomicU32 = AtomicU32::new(1);

const PREFIX: &[u8] = b"\x1b_G";
const SUFFIX: &[u8] = b"\x1b\\";
const PLACEHOLDER: &str = "\u{10EEEE}";

pub struct StivImage {
    pub path: String,
    width_px: u16,
    height_px: u16,
    size_cols: u16,
    pub cell_width_px: u16,
    pub cell_height_px: u16,
    size_rows: u16,
    pos_col: u16,
    pos_row: u16,
    id: u32,
    pub uploaded: bool,
    pub last_area: Option<Rect>,
    pub zoom_state: f32,
    original_image: DynamicImage,
    displayed_image: DynamicImage,
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
            original_image: img.clone(),
            displayed_image: img,
            shm_file: shm_file,
        })
    }

    pub fn resize_to_fit(&mut self, area: &Rect) {
        let new_width = (area.width * self.cell_width_px) as u32;
        let new_height = (area.height * self.cell_height_px) as u32;

        if new_width > self.width_px as u32 &&
            new_height > self.height_px as u32 &&
            !self.uploaded &&
            self.zoom_state == 1.0 {
            return
        }

        let new_img_width  = (new_width  as f32 * self.zoom_state) as u32;
        let new_img_height = (new_height as f32 * self.zoom_state) as u32;

        let src_rgb = self.original_image.to_rgb8();

        let src = fir::images::ImageRef::new(
            src_rgb.width(),
            src_rgb.height(),
            src_rgb.as_raw(),
            fir::PixelType::U8x3,
        ).unwrap();

        let mut dst = FirImage::new(new_img_width, new_img_height, fir::PixelType::U8x3);

        let mut resizer = fir::Resizer::new();

        resizer.resize(
            &src,
            &mut dst,
            &fir::ResizeOptions::new()
                //.resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3)),
                .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Box)),
        ).unwrap();

        let rgb_image = image::RgbImage::from_raw(
            new_img_width,
            new_img_height,
            dst.into_vec(),
        ).unwrap();

        self.displayed_image = DynamicImage::ImageRgb8(rgb_image);
        self.uploaded = false;

        ()
        //return Rect::new(area.x + (width_offset_in_cols / 2), area.y + (height_offset_in_rows / 2), new_width_in_cols, new_height_in_rows);
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
        let img = self.displayed_image.clone();
        let img_rgb = img.to_rgb8();
        let width  = img_rgb.width();
        let height = img_rgb.height();
        let raw    = img_rgb.as_raw();

        let encoded = BASE64_STANDARD.encode(raw);
        let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(4096).collect();
        let last_idx = chunks.len().saturating_sub(1);

        let id = self.id;
        let mut stdout = io::stdout();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_first = i == 0;
            let is_last  = i == last_idx;
            let m = if is_last { 0 } else { 1 };

            let control = if is_first {
                // All metadata on the first chunk only
                //format!("a=T,f=24,t=d,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2,m={m}")
                // This is ugly af, but it's just try
                // Eventually, we want the image to scale such that if the terminal allows is to
                // grow vertically, even if the aspect ratio has limited it in the horizontal axis,
                // and vice versa
                if self.zoom_state != 1.0 {
                    format!("a=T,f=24,t=d,C=1,U=1,i={},s={},v={},x={},y={},w={},h={},q=2,m={}", id, width, height, area.x, area.y, area.width, area.height, m)
                } else {
                    format!("a=T,f=24,t=d,C=1,U=1,i={id},s={width},v={height},q=2,m={m}")
                }
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
        //self.last_area = Some(*area);

        Ok(())
    }

    pub fn upload_shm(&mut self, area: &Rect) -> anyhow::Result<()> {
        let img = self.displayed_image.clone();
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
            let rows = area.height - 1;
            let cols = area.width;
            //log::info!("in upload_shm: cols, rows: {}, {}", cols, rows);

            //let cmd = format!(
            //    "\x1b_Ga=T,f=24,t=s,s={width},v={height},q=2;{path_b64}\x1b\\",
            //);

            let cmd = match self.uploaded {
                true => format!("\x1b_Ga=p,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2\x1b\\"),
                false => format!("\x1b_Ga=T,f=24,t=s,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2;{path_b64}\x1b\\")

            };

            let mut stdout = io::stdout();
            stdout.write_all(cmd.as_bytes())?;
            stdout.flush()?;

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

    pub fn render_placeholders_without_ratatui_buf(&self, area: Rect) {
        let id = self.id;
        let mut stdout = io::stdout();

        // Encode image ID as an RGB foreground color:
        // red   = (id >> 16) & 0xff
        // green = (id >>  8) & 0xff
        // blue  =  id        & 0xff
        let r = ((id >> 16) & 0xff) as u8;
        let g = ((id >>  8) & 0xff) as u8;
        let b = (id         & 0xff) as u8;

        let _ = stdout.write_all(format!("\x1b[38;2;{};{};{}m", r, g, b).as_bytes());

        for row in 0..area.height {
            let _ = stdout.write_all(format!("\x1b[{};{}H", area.y + row + 1, area.x + 1).as_bytes());
            for col in 0..area.width {
                let row_diacritic = kitty_diacritics::diacritic_for_index(row as u32);
                let col_diacritic = kitty_diacritics::diacritic_for_index(col as u32);

                let _ = stdout.write_all(format!(
                    "{}{}{}", PLACEHOLDER, row_diacritic, col_diacritic
                ).as_bytes());
            }
        }
    }

    pub fn render_direct_transmission(&mut self) -> anyhow::Result<()> {
        let img = self.displayed_image.clone();
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
        let img = self.displayed_image.clone();
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

        Ok(())
    }

    fn adjust_for_aspect_ratio(&self, new_width: u32, new_height: u32) -> (u32, u32) {
        let ratio = self.original_image.width() as f32 / self.original_image.height() as f32;
        let test_width = new_height as f32 * ratio;

        if test_width > new_width as f32 {
            let height = new_width as f32 / ratio;
            // round the result?
            return (new_width, height as u32);

        }

        // round the result?
        return (test_width as u32, new_height);
    }

    fn get_area_adjusted_for_aspect_ratio(&self, area: &Rect) -> Rect {
        let mut adjusted_area = area.clone();
        let ratio = self.original_image.width() as f32 / self.original_image.height() as f32;
        log::info!("ratio: {}", ratio);
        log::info!("img height px: {}", self.original_image.height());
        let area_width_px = (area.width * self.cell_width_px) as f32;
        let area_height_px = (area.height * self.cell_height_px) as f32;

        let test_width_px = area_height_px * ratio;

        if test_width_px > area_width_px as f32 {
            let height_px = area_width_px / ratio;

            adjusted_area.height = height_px as u16 / self.cell_height_px;
        } else {
            adjusted_area.width = test_width_px as u16 / self.cell_width_px;
        }

        let width_offset_in_cols = area.width.saturating_sub(adjusted_area.width);
        let height_offset_in_rows = area.height.saturating_sub(adjusted_area.height);

        adjusted_area.x = area.x.saturating_add(width_offset_in_cols / 2);
        adjusted_area.y = area.y.saturating_add(height_offset_in_rows / 2);

        adjusted_area
    }

    pub fn delete_from_terminal(&mut self) {
        if !self.uploaded { return; }
        let id = self.id;
        let cmd = format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
        let mut stdout = io::stdout();
        let _ = stdout.write_all(cmd.as_bytes());
        let _ = stdout.flush();
        self.uploaded = false;
        self.last_area = None;
    }
}

pub struct StivImageWidget {
    pub current_event: StivEvent,
}

impl StatefulWidget for StivImageWidget {
    type State = StivImage;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut StivImage) {
        // so, what do we need to do?
        // if image is not uploaded, we prolly need to resize it (for the first time)
        // if event is resize in single image mode, we need to resize it
        // if event is toggle_mode, we need to resize it (of even save one instance of each size?
        //let mut new_area = area.clone();

        // resized area is always different from this, because of aspect ratio. We might need two
        // separate areas saved in the stiv_img instance. One for this area, one for the
        // aspect-ratio-adjusted one
        log::info!("before adjusted new_area x,y, w,h {},{}, {},{}", area.x, area.y, area.width, area.height);
        let new_area = state.get_area_adjusted_for_aspect_ratio(&area);
        log::info!("after adjusted new_area x,y, w,h {},{}, {},{}", area.x, area.y, area.width, area.height);
        log::info!("cell_height_px: {}", state.cell_height_px);

        // Maybe call adjust_for_aspect_ratio before this, separately, and compare against
        // last_adjusted_area!
        let area_size_changed = match state.last_area {
            Some(last_area) => {
                (new_area.width, new_area.height) != (last_area.width, last_area.height)
            },
            None => false
        };

        // area_size_changed is always true!???

        if !state.uploaded || area_size_changed {
            state.resize_to_fit(&new_area);
        }

        // if event is zoom, we do a separate, other rescale

        // we only need to upload if:
        // a) it hasn't been uploaded for the first time
        // b) we have done a source image resize
        if !state.uploaded {
            log::info!("upload called for {}", state.path);
            if let Err(e) = state.upload_stream(&new_area) {
                log::error!("upload error: {e}");
                return;
            }
        }

        state.render_placeholders(new_area, buf);
        state.last_area = Some(area);
    }
}
