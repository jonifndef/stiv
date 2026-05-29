use std::io::{self, Write};

use anyhow::Error;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::{detect_support, stiv_image::{self, StivImage}};


trait Transport: Send {
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer) -> anyhow::Result<String>;
}

struct DirectStreamTransport;

impl Transport for DirectStreamTransport {
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer) -> anyhow::Result<String> {

        Ok(String::from(""))
    }
}

struct TmpFileTransport;

impl Transport for TmpFileTransport {
    fn get_upload_string(&self, stiv_image: &mut StivImage, area: &Rect, buf: &mut Buffer) -> anyhow::Result<String> {

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
            if let Ok(upload_string) = self.transport.get_upload_string(stiv_image, &new_area, buf) {
                let mut stdout = io::stdout();
                stdout.write_all(upload_string.as_bytes())?;
                stdout.flush()?;
            } else {
                return Err(anyhow::anyhow!(""));
            }
        }

        stiv_image.render_placeholders(new_area, buf);

        Ok(())
    }
}
