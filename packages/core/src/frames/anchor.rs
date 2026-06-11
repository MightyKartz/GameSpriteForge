use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FootAnchor {
    pub x: f32,
    pub y: f32,
    pub locked_by_user: bool,
}

pub fn default_foot_anchor(
    canvas_width: u32,
    canvas_height: u32,
    margin_bottom: u32,
) -> FootAnchor {
    FootAnchor {
        x: canvas_width as f32 / 2.0,
        y: canvas_height.saturating_sub(margin_bottom) as f32,
        locked_by_user: false,
    }
}

pub fn manual_anchor(x: f32, y: f32) -> FootAnchor {
    FootAnchor {
        x,
        y,
        locked_by_user: true,
    }
}
