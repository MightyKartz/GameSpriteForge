use std::env;
use std::path::PathBuf;

use forge_core::asset_project::{
    assess_character_canonical_identity_reference, assess_character_identity_reference,
    normalize_matted_static_image,
};
use forge_core::character_direction::{assess_direction_anchor, DirectionViewV1};
use forge_core::keyframe_cleanup::cleanup_keyframe_background;

fn main() {
    let mut args = env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .expect(
                "usage: inspect_direction_anchor <png> <front|rear|right> [cleaned-output] [normalized-output]",
            ),
    );
    let direction = match args.next().as_deref() {
        Some("front") => DirectionViewV1::Front,
        Some("rear") => DirectionViewV1::Rear,
        Some("right") => DirectionViewV1::Right,
        _ => panic!("direction must be front, rear, or right"),
    };
    let image = image::open(path).expect("read PNG").to_rgba8();
    let (mut image, cleanup) = if let Some(output) = args.next() {
        let (cleaned, report) = cleanup_keyframe_background(&image);
        cleaned.save(output).expect("write cleaned PNG");
        (cleaned, Some(report))
    } else {
        (image, None)
    };
    if let Some(output) = args.next() {
        image = normalize_matted_static_image(&image, &PathBuf::from(output), 256, true)
            .expect("normalize cleaned PNG");
    }
    let identity_report = assess_character_identity_reference(&image, "human ranger");
    let canonical_identity_report =
        assess_character_canonical_identity_reference(&image, "human ranger");
    let direction_report = assess_direction_anchor(&image, "human ranger", direction);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "cleanup": cleanup,
            "identity": identity_report,
            "canonicalIdentity": canonical_identity_report,
            "direction": direction_report,
        }))
        .expect("serialize report")
    );
}
