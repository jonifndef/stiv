pub enum ImageFileType {
    Jpeg,
    Png,
}

pub fn get_image_type(_image_filename: &str) -> ImageFileType {
    ImageFileType::Jpeg
}
