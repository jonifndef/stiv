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
use tempfile::NamedTempFile;
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
    pub id: u32,
    pub uploaded: bool,
    pub last_area: Option<Rect>,
    pub zoom_state: f32,
    original_image: DynamicImage,
    pub displayed_image: DynamicImage,
    shm_file: Option<ShmFile>,
    pub tmp_file: Option<NamedTempFile>,
    pub crop_area: Option<ImgRect>
}

#[derive(Default, Clone)]
pub struct ImgRect {
    pub x_px: u32,
    pub y_px: u32,
    pub width_px: u32,
    pub height_px: u32
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
            tmp_file: None,
            crop_area: None,
        })
    }

    fn resize(&mut self, new_width_px: u32, new_height_px: u32) -> anyhow::Result<()> {
        let src_rgb = self.original_image.to_rgb8();
        //let src_rgb = self.displayed_image.to_rgb8(); this might be faster, I dunno. But it looks worse

        let src = fir::images::ImageRef::new(
            src_rgb.width(),
            src_rgb.height(),
            src_rgb.as_raw(),
            fir::PixelType::U8x3,
        )?;

        let mut dst = FirImage::new(new_width_px, new_height_px, fir::PixelType::U8x3);

        let mut resizer = fir::Resizer::new();

        resizer.resize(
            &src,
            &mut dst,
            &fir::ResizeOptions::new()
                .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3)),
        )?;

        let rgb_image = image::RgbImage::from_raw(
            new_width_px,
            new_height_px,
            dst.into_vec(),
        ).unwrap();

        self.displayed_image = DynamicImage::ImageRgb8(rgb_image);
        self.uploaded = false;
        self.width_px = self.displayed_image.width() as u16;
        self.height_px = self.displayed_image.height() as u16;

        Ok(())
    }

    // TODO: Add return type, remove unwrap()
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

        self.resize(new_img_width, new_img_height).unwrap();

        ()
    }

    #[allow(unused)]
    pub fn upload_shm(&mut self, area: &Rect) -> anyhow::Result<()> {
        let img = self.displayed_image.clone();
        let img_rgb = img.into_rgb8();
        let width = img_rgb.width();
        let height = img_rgb.height();
        let img_rgb_raw = img_rgb.into_raw();

        // Make this more obvious, somwthing like "if shm_available()"
        if let Some(shm_file) = &mut self.shm_file {
            shm_file.resize_if_needed(img_rgb_raw.len())?;
            shm_file.write_to_shm_file(&img_rgb_raw)?;

            let path_b64 = BASE64_STANDARD.encode(shm_file.get_shm_path());
            let id = self.id;
            let rows = area.height - 1;
            let cols = area.width;

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

    pub fn get_display_area_for_zoomed_img(&mut self, area: &Rect) -> Rect {
        let mut cols = self.width_px.div_ceil(self.cell_width_px);
        let mut rows = self.height_px.div_ceil(self.cell_height_px);

        if cols > area.width {
            cols = area.width;
        }
        if rows > area.height {
            rows = area.height;
        }

        let x = area.x + area.width.saturating_sub(cols) / 2;
        let y = area.y + area.height.saturating_sub(rows) / 2;

        Rect {
            x: x,
            y: y,
            width: cols,
            height: rows
        }
    }

    pub fn get_area_adjusted_for_aspect_ratio(&self, area: &Rect) -> Rect {
        let mut adjusted_area = area.clone();
        let ratio = self.original_image.width() as f32 / self.original_image.height() as f32;
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

    pub fn get_crop_area_for_zoomed_img(&mut self, area: &Rect) -> anyhow::Result<ImgRect> {
        let x_offset = self.displayed_image.width().saturating_sub(area.width as u32 * self.cell_width_px as u32) / 2;
        let y_offset = self.displayed_image.height().saturating_sub(area.height as u32 * self.cell_height_px as u32) / 2;
        log::info!("img h: {}, area h: {}", self.displayed_image.height(), area.height * self.cell_height_px);

        let crop_area = ImgRect {
            x_px: x_offset,
            y_px: y_offset,
            width_px: (area.width * self.cell_width_px) as u32,
            height_px: (area.height * self.cell_height_px) as u32,
        };

        self.crop_area = Some(crop_area.clone());

        return Ok(crop_area);
    }

    pub fn crop(&mut self, area: &Rect) -> anyhow::Result<()> {
        let display_px_w = (area.width  * self.cell_width_px)  as u32;
        let display_px_h = (area.height * self.cell_height_px) as u32;

        let x = self.displayed_image.width().saturating_sub(display_px_w)  / 2;
        let y = self.displayed_image.height().saturating_sub(display_px_h) / 2;

        let crop_w = display_px_w.min(self.displayed_image.width()  - x);
        let crop_h = display_px_h.min(self.displayed_image.height() - y);

        self.displayed_image = self.displayed_image.crop_imm(x, y, crop_w, crop_h);

        Ok(())
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

    pub fn resize_zoom_in(&mut self) -> anyhow::Result<()> {
        self.zoom_state += 0.15;

        let new_width  = (self.displayed_image.width()  as f32 * self.zoom_state) as u32;
        let new_height = (self.displayed_image.height() as f32 * self.zoom_state) as u32;

        self.resize(new_width, new_height)?;

        Ok(())
    }

    pub fn get_zoom_crop_area_px(&mut self) -> Rect {

        Rect::default()
    }
}
