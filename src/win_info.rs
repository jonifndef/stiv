use crossterm::terminal::{self, window_size, WindowSize};

pub struct WinInfo {
    pub width_px: u16,
    pub height_px: u16,
    pub cols: u16,
    pub rows: u16,
    pub cell_height: u16,
    pub cell_width: u16
}

impl WinInfo {
    pub fn get_win_info() -> Result<Self, anyhow::Error> {
        let window_size = terminal::window_size()?;

        let info = WinInfo {
            width_px:    window_size.width,
            height_px:   window_size.height,
            cols:        window_size.columns,
            rows:        window_size.rows,
            cell_height: (window_size.width  / window_size.columns),
            cell_width:  (window_size.height / window_size.rows)
        };

        Ok(info)
    }
}
