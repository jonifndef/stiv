#[derive(Copy, Clone, PartialEq)]
pub enum StivEvent {
    None,
    TermResize,
    ZoomIn,
    ZoomOut,
    SingleImageMode,
    GalleryMode,
    // pan events, etc etc
}
