use std::io::{Seek, SeekFrom};
use std::io::{self, Write};

use anyhow::Error;
use anyhow::anyhow;
use base64::{prelude::BASE64_STANDARD, Engine};
use ratatui::{buffer::Buffer, layout::Rect};
use tempfile::NamedTempFile;

use crate::stiv_event::StivEvent;
use crate::{detect_support, stiv_image::{StivImage}};

const DEFAULT_START_SEQUENCE: &str = "\x1b_G";
const DEFAULT_END_SEQUENCE:   &str = "\x1b\\";
const TMUX_START_SEQUENCE:    &str = "\x1bPtmux;\x1b\x1b_G";
const TMUX_END_SEQUENCE:      &str = "\x1b\x1b\\\x1b\\";
const UNICODE_PLACEHOLDER:    &str = "\u{10EEEE}";

trait Transport: Send {
    fn upload(&self, stiv_image: &mut StivImage, renderer: &Renderer) -> anyhow::Result<()>;
}

struct DirectStreamTransport;

impl Transport for DirectStreamTransport {
    fn upload(&self, stiv_img: &mut StivImage, renderer: &Renderer) -> anyhow::Result<()> {
        let img = stiv_img.displayed_image.clone();
        let img_rgb = img.to_rgb8();
        let width  = img_rgb.width();
        let height = img_rgb.height();
        let raw    = img_rgb.as_raw();

        let encoded = BASE64_STANDARD.encode(raw);
        let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(4096).collect();
        let last_idx = chunks.len().saturating_sub(1);

        let id = stiv_img.id;
        let mut stdout = io::stdout();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_first = i == 0;
            let is_last  = i == last_idx;
            let m = if is_last { 0 } else { 1 };

            let control = if is_first {
                format!("a=T,f=24,t=d,C=1,U=1,i={id},s={width},v={height},q=2,m={m}")
            } else {
                format!("m={m},i={id},q=2")
            };

            let mut out = Vec::new();
            out.extend_from_slice(renderer.start_escape_sequence.as_bytes());
            out.extend_from_slice(control.as_bytes());
            out.push(b';');
            out.extend_from_slice(chunk);
            out.extend_from_slice(renderer.end_escape_sequence.as_bytes());
            stdout.write_all(&out)?;
        }

        stdout.flush()?;
        stiv_img.uploaded = true;

        Ok(())
    }
}

struct TmpFileTransport;

impl Transport for TmpFileTransport {
    fn upload(&self, stiv_img: &mut StivImage, renderer: &Renderer) -> anyhow::Result<()> {
        let img = stiv_img.displayed_image.clone();
        let img_rgb = img.to_rgb8();
        let width  = img_rgb.width();
        let height = img_rgb.height();
        let raw    = img_rgb.as_raw();

        let tmp_file: &mut NamedTempFile = stiv_img.tmp_file.get_or_insert_with(|| {
            NamedTempFile::new().expect("failed to create temp file")
        });

        tmp_file.seek(SeekFrom::Start(0))?;
        tmp_file.as_file().set_len(0)?;
        tmp_file.write_all(raw)?;
        tmp_file.flush()?;

        let path_str = tmp_file.path().to_str().ok_or_else(|| anyhow!("path is not valid UTF-8"))?;
        let encoded_path = BASE64_STANDARD.encode(path_str);

        let id = stiv_img.id;
        let mut stdout = io::stdout();

        let mut data = String::from("");
        data.push_str(renderer.start_escape_sequence);
        data.push_str(format!("a=T,f=24,t=f,C=1,U=1,i={},s={},v={},q=2;{}", id, width, height, encoded_path).as_str());
        data.push_str(renderer.end_escape_sequence);

        stdout.write_all(data.as_bytes())?;
        stdout.flush()?;
        stiv_img.uploaded = true;

        Ok(())
    }
}

struct ShmTransport;

pub struct Renderer {
    transport: Box<dyn Transport>,
    is_tmux: bool,
    start_escape_sequence: &'static str,
    end_escape_sequence: &'static str,
}

impl Renderer {
    pub fn new() -> Self {
        let is_tmux = detect_support::is_tmux();
        Self {
            transport: if detect_support::is_ssh() {
                Box::new(DirectStreamTransport)
            } else {
                Box::new(TmpFileTransport)
            },
            is_tmux: is_tmux,
            start_escape_sequence: if is_tmux { TMUX_START_SEQUENCE } else { DEFAULT_START_SEQUENCE },
            end_escape_sequence: if is_tmux { TMUX_END_SEQUENCE } else { DEFAULT_END_SEQUENCE },
        }
    }

    pub fn render(&mut self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer, current_event: &StivEvent) -> anyhow::Result<()> {

        let new_area: Rect;

        match *current_event {
            StivEvent::ZoomIn => {
                stiv_image.resize_zoom_in()?;

                new_area = stiv_image.get_display_area_for_zoomed_img(&area);
                let crop_area = stiv_image.get_crop_area_for_zoomed_img(&new_area)?;

                {
                    let img = stiv_image.displayed_image.clone();
                    let img_rgb = img.to_rgb8();
                    let width  = img_rgb.width();
                    let height = img_rgb.height();
                    let raw    = img_rgb.as_raw();

                    let tmp_file: &mut NamedTempFile = stiv_image.tmp_file.get_or_insert_with(|| {
                        NamedTempFile::new().expect("failed to create temp file")
                    });

                    tmp_file.seek(SeekFrom::Start(0))?;
                    tmp_file.as_file().set_len(0)?;
                    tmp_file.write_all(raw)?;
                    tmp_file.flush()?;

                    let path_str = tmp_file.path().to_str().ok_or_else(|| anyhow!("path is not valid UTF-8"))?;
                    let encoded_path = BASE64_STANDARD.encode(path_str);

                    let id = stiv_image.id;
                    let mut stdout = io::stdout();

                    let mut data = String::from("");
                    data.push_str(self.start_escape_sequence);
                    //data.push_str(format!("a=T,f=24,t=f,U=1,i={},s={},v={},q=2;{}", id, width, height, encoded_path).as_str());
                    data.push_str(format!("a=t,f=24,t=f,U=1,C=1,i={},s={},v={},c={},r={},q=2;{}", id, width, height, new_area.width, new_area.height, encoded_path).as_str());
                    data.push_str(self.end_escape_sequence);

                    stdout.write_all(data.as_bytes())?;
                    stdout.flush()?;
                    stiv_image.uploaded = true;

                    data.clear();

                    data.push_str(self.start_escape_sequence);
                    data.push_str(format!("a=p,U=1,i={},x={},y={},w={},h={},c={},r={}", id, crop_area.x_px, crop_area.y_px, crop_area.width_px, crop_area.height_px, new_area.width, new_area.height).as_str());
                    data.push_str(self.end_escape_sequence);

                    stdout.write_all(data.as_bytes())?;
                    stdout.flush()?;
                }
            },
            _ => {
                new_area = stiv_image.get_area_adjusted_for_aspect_ratio(&area);

                let area_size_changed = match stiv_image.last_area {
                    Some(last_area) => {
                        (new_area.width, new_area.height) != (last_area.width, last_area.height)
                    },
                    None => false
                };

                if !stiv_image.uploaded || area_size_changed {
                    stiv_image.resize_to_fit(&new_area);
                }

                if !stiv_image.uploaded {
                    self.transport.upload(stiv_image, &self)?;
                    stiv_image.last_area = Some(new_area);
                }
            }
        }

        stiv_image.render_placeholders(new_area, buf);

        Ok(())
    }
}
