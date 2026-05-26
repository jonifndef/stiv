#[derive(Copy, Clone, PartialEq)]
pub enum StivEvent {
    None,
    TermResize,
    ZoomIn,
    ZoomOut,
    ToggleMode,
    // pan events, etc etc
}
