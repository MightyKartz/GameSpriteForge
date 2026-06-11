pub mod extract;
pub mod ffmpeg;
pub mod probe;
pub mod sprite_sheet;

pub use extract::{extract_frames, ExtractFramesParams, ExtractFramesResult};
pub use ffmpeg::{
    find_in_path, resolve_binary, resolve_ffmpeg_paths, FfmpegPaths, FfmpegSearch,
    FFMPEG_MISSING_CODE, FFMPEG_MISSING_MESSAGE,
};
pub use probe::{probe_video, ProbeVideoParams, VideoProbe};
pub use sprite_sheet::{
    slice_sprite_sheet_grid, slice_sprite_sheet_transparent, SliceSpriteSheetParams,
    SliceSpriteSheetTransparentParams,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoError {
    pub code: String,
    pub message: String,
}

impl VideoError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn ffmpeg_missing() -> Self {
        Self::new(ffmpeg::FFMPEG_MISSING_CODE, ffmpeg::FFMPEG_MISSING_MESSAGE)
    }

    pub fn command_failed(message: impl Into<String>) -> Self {
        Self::new("ffmpeg_command_failed", message)
    }

    pub fn invalid_output(message: impl Into<String>) -> Self {
        Self::new("ffmpeg_invalid_output", message)
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new("video_invalid_params", message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new("video_io", message)
    }
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for VideoError {}

impl From<std::io::Error> for VideoError {
    fn from(error: std::io::Error) -> Self {
        Self::io(error.to_string())
    }
}

impl From<serde_json::Error> for VideoError {
    fn from(error: serde_json::Error) -> Self {
        Self::invalid_output(error.to_string())
    }
}
