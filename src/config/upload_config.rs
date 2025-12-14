use super::default_true;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UploadConfig {
    #[serde(default = "default_upload_path")]
    pub path: String,
    #[serde(default = "default_max_image_size_mb")]
    pub max_image_size_mb: u64,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    #[serde(default = "default_true")]
    pub enable_image_processing: bool,
    #[serde(default = "default_image_quality")]
    #[allow(dead_code)]
    pub image_quality: u8,
    #[serde(default = "default_thumbnail_max_width")]
    pub thumbnail_max_width: u32,
    #[serde(default = "default_thumbnail_max_height")]
    pub thumbnail_max_height: u32,
    #[serde(default = "default_true")]
    pub save_original: bool,
}

fn default_upload_path() -> String {
    "uploads".to_string()
}

fn default_max_image_size_mb() -> u64 {
    10
}

fn default_max_file_size_mb() -> u64 {
    50
}

fn default_image_quality() -> u8 {
    85
}

fn default_thumbnail_max_width() -> u32 {
    800
}

fn default_thumbnail_max_height() -> u32 {
    800
}
