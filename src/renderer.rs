use std::io::{self, Write};

use anyhow::Error;
use base64::{prelude::BASE64_STANDARD, Engine};
use ratatui::{buffer::Buffer, layout::Rect};

use crate::{detect_support, stiv_image::{self, StivImage}};


trait Transport: Send {
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect) -> anyhow::Result<String>;
}

struct DirectStreamTransport;

impl Transport for DirectStreamTransport {
    // For direct stream, we cannot return a single image and write to stdout, we need to write
    // multiple chunks to stdout, so the renderer.render function needs to change accordingly
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect) -> anyhow::Result<String> {
        let img = stiv_image.displayed_image.clone();
        let img_rgb = img.to_rgb8();
        let width  = img_rgb.width();
        let height = img_rgb.height();
        let raw    = img_rgb.as_raw();

        let encoded = BASE64_STANDARD.encode(raw);
        let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(4096).collect();
        let last_idx = chunks.len().saturating_sub(1);

        let id = stiv_image.id;
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
                if stiv_image.zoom_state != 1.0 {
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
        stiv_image.uploaded = true;
        stiv_image.last_area = Some(*area);
        log::info!("in upload, setting last_area w,h to {},{}", area.width, area.height);

        Ok(String::from(""))
    }
}

struct TmpFileTransport;

impl Transport for TmpFileTransport {
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect) -> anyhow::Result<String> {

        Ok(String::from(""))
    }
}

struct ShmTransport;


pub struct Renderer {
    transport: Box<dyn Transport>,
    is_tmux: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            transport: if detect_support::is_ssh() {
                Box::new(DirectStreamTransport)
            } else {
                Box::new(TmpFileTransport)
            },
            is_tmux: detect_support::is_tmux()
        }
    }

    pub fn render(&mut self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer) -> anyhow::Result<()> {
        //let new_area = stiv_image.get_area_adjusted_for_aspect_ratio(&area);
        //log::info!("new_area w,h: {},{}", new_area.width, new_area.height);

        //let area_size_changed = match stiv_image.last_area {
        //    Some(last_area) => {
        //        log::info!("last_area w,h: {},{}", last_area.width, last_area.height);
        //        (new_area.width, new_area.height) != (last_area.width, last_area.height)
        //    },
        //    None => false
        //};

        //if !stiv_image.uploaded || area_size_changed {
        //    stiv_image.resize_to_fit(&new_area);
        //}

        //// if event is zoom, we do a separate, other rescale

        //if !stiv_image.uploaded {
        //    if let Ok(upload_string) = self.transport.get_upload_string(stiv_image, &new_area, buf) {
        //        let mut stdout = io::stdout();
        //        stdout.write_all(upload_string.as_bytes())?;
        //        stdout.flush()?;
        //    } else {
        //        return Err(anyhow::anyhow!(""));
        //    }
        //}

        //stiv_image.render_placeholders(new_area, buf);

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
            if let Ok(upload_string) = self.transport.get_upload_string(stiv_image, &new_area) {
                log::info!("{}", upload_string);
            } else {
                return Err(anyhow::anyhow!(""));
            }
            //if let Err(e) = stiv_image.upload_stream(&new_area) {
            //    log::error!("upload error: {e}");
            //    return Err(anyhow::anyhow!(""));
            //}
        }

        stiv_image.render_placeholders(new_area, buf);

        Ok(())
    }
}
