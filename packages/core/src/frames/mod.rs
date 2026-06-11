pub mod anchor;
pub mod bbox;
pub mod normalize;

pub use anchor::{default_foot_anchor, manual_anchor, FootAnchor};
pub use bbox::{bbox_from_alpha_rgba, bbox_from_image, FrameBbox, FrameSize};
pub use normalize::{
    normalize_frames, CanvasMode, NormalizeOptions, NormalizeWarning, NormalizedFrame,
};
