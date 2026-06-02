use std::io::{Seek, SeekFrom};
use std::io::{self, Write};

use anyhow::Error;
use anyhow::anyhow;
use base64::{prelude::BASE64_STANDARD, Engine};
use ratatui::{buffer::Buffer, layout::Rect};
use tempfile::NamedTempFile;

use crate::{detect_support, stiv_image::{StivImage}};

const DEFAULT_START_SEQUENCE: &str = "\x1b_G";
const DEFAULT_END_SEQUENCE:   &str = "\x1b\\";
const TMUX_START_SEQUENCE:    &str = "\x1bPtmux;\x1b\x1b_G";
const TMUX_END_SEQUENCE:      &str = "\x1b\x1b\\\x1b\\";
const UNICODE_PLACEHOLDER:    &str = "\u{10EEEE}";

trait Transport: Send {
    fn upload(&self, stiv_image: &mut StivImage, area: &Rect, renderer: &Renderer) -> anyhow::Result<()>;
}

struct DirectStreamTransport;

impl Transport for DirectStreamTransport {
    fn upload(&self, stiv_img: &mut StivImage, area: &Rect, renderer: &Renderer) -> anyhow::Result<()> {
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
                // All metadata on the first chunk only
                //format!("a=T,f=24,t=d,U=1,i={id},c={cols},r={rows},s={width},v={height},q=2,m={m}")
                // This is ugly af, but it's just try
                // Eventually, we want the image to scale such that if the terminal allows is to
                // grow vertically, even if the aspect ratio has limited it in the horizontal axis,
                // and vice versa
                if stiv_img.zoom_state != 1.0 {
                    format!("a=T,f=24,t=d,C=1,U=1,i={},s={},v={},x={},y={},w={},h={},q=2,m={}", id, width, height, area.x, area.y, area.width, area.height, m)
                } else {
                    format!("a=T,f=24,t=d,C=1,U=1,i={id},s={width},v={height},q=2,m={m}")
                }
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
        stiv_img.last_area = Some(*area);
        log::info!("in upload, setting last_area w,h to {},{}", area.width, area.height);

        Ok(())
    }
}

struct TmpFileTransport;

impl Transport for TmpFileTransport {
    fn upload(&self, stiv_img: &mut StivImage, area: &Rect, renderer: &Renderer) -> anyhow::Result<()> {
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

        log::info!("data string: {}", data);
        stdout.write_all(data.as_bytes())?;
        stdout.flush()?;
        stiv_img.uploaded = true;
        stiv_img.last_area = Some(*area);
        log::info!("in upload, setting last_area w,h to {},{}", area.width, area.height);

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

    pub fn render(&mut self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer) -> anyhow::Result<()> {
        let new_area = stiv_image.get_area_adjusted_for_aspect_ratio(&area);
        log::info!("new_area w,h: {},{}", new_area.width, new_area.height);

        let area_size_changed = match stiv_image.last_area {
            Some(last_area) => {
                log::info!("last_area w,h: {},{}", last_area.width, last_area.height);
                (new_area.width, new_area.height) != (last_area.width, last_area.height)
            },
            None => false
        };

        if !stiv_image.uploaded || area_size_changed {
            stiv_image.resize_to_fit(&new_area);
        }

        // if event is zoom, we do a separate, other rescale

        if !stiv_image.uploaded {
            self.transport.upload(stiv_image, &new_area, &self)?;
        }

        stiv_image.render_placeholders(new_area, buf);

        Ok(())
    }
}

pub fn get_tmux_header(tmux_nest_count: u32) -> String {
    let mut header: String = String::new();
    for i in 0..tmux_nest_count {
        header.push_str(&"\u{1b}".repeat(2usize.pow(i)));
        header.push_str("Ptmux;");
    }
    header
}

pub fn get_tmux_tail(tmux_nest_count: u32) -> String {
    let mut tail: String = String::new();
    for i in (0..tmux_nest_count).rev() {
        tail.push_str(&"\u{1b}".repeat(2usize.pow(i)));
        tail.push('\\');
    }
    tail
}
