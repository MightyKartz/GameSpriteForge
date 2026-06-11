pub mod chroma;

pub use chroma::{
    alpha_bbox, apply_chroma_key, process_chroma_batch, process_chroma_frame, AlphaBBox,
    BatchMattingResult, ChromaKeyMode, ChromaParameters, FrameBBox, MattingError, MattingResult,
};
