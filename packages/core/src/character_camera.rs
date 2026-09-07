use serde::{Deserialize, Serialize};

/// Versioned camera contracts for generated top-down Character assets.
///
/// The legacy profile preserves the perspective used by existing Jobs. New
/// stable video specs must select a profile explicitly so cardinal and
/// three-quarter assets cannot be mixed silently in one Pack.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterCameraProfileV1 {
    #[default]
    #[serde(rename = "topdown-3q-orthographic@1.0.0")]
    LegacyThreeQuarter,
    #[serde(rename = "topdown-three-quarter@1.0.0")]
    TopdownThreeQuarter,
    #[serde(rename = "topdown-orthographic@2.0.0")]
    TopdownOrthographic,
}

impl CharacterCameraProfileV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyThreeQuarter => "topdown-3q-orthographic@1.0.0",
            Self::TopdownThreeQuarter => "topdown-three-quarter@1.0.0",
            Self::TopdownOrthographic => "topdown-orthographic@2.0.0",
        }
    }

    pub const fn prompt_contract(self) -> &'static str {
        match self {
            Self::LegacyThreeQuarter | Self::TopdownThreeQuarter => {
                "fixed top-down three-quarter orthographic camera with a 45-degree pitch; preserve the same pitch and cardinal facing in every frame"
            }
            Self::TopdownOrthographic => {
                "fixed cardinal orthographic RPG sprite camera with no three-quarter yaw; front, rear, and right are axis-aligned views with identical pitch and scale"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_profiles_have_stable_public_ids() {
        assert_eq!(
            serde_json::to_value(CharacterCameraProfileV1::TopdownOrthographic).unwrap(),
            "topdown-orthographic@2.0.0"
        );
        assert_eq!(
            serde_json::to_value(CharacterCameraProfileV1::TopdownThreeQuarter).unwrap(),
            "topdown-three-quarter@1.0.0"
        );
        assert_eq!(
            serde_json::to_value(CharacterCameraProfileV1::LegacyThreeQuarter).unwrap(),
            "topdown-3q-orthographic@1.0.0"
        );
    }
}
