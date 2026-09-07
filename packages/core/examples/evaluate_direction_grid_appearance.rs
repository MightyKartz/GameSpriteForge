use std::collections::BTreeMap;
use std::collections::{BTreeSet, VecDeque};
use std::env;
use std::error::Error;
use std::path::PathBuf;

use forge_core::asset_project::{character_body_bbox, image_signature, CharacterEquipmentKindV1};
use forge_core::character_grid::assess_direction_grid_appearance;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 6 {
        return Err(
            "usage: evaluate_direction_grid_appearance <none|staff_like> <canonical> <front> <back> <right> <left>"
                .into(),
        );
    }
    let equipment = match arguments[0].as_str() {
        "none" => CharacterEquipmentKindV1::None,
        "staff_like" => CharacterEquipmentKindV1::StaffLike,
        other => return Err(format!("unsupported equipment kind: {other}").into()),
    };
    let canonical = image::open(PathBuf::from(&arguments[1]))?.to_rgba8();
    let nodes = ["front_idle", "back_idle", "right_idle", "left_idle"]
        .into_iter()
        .zip(arguments[2..].iter())
        .map(|(node_id, path)| {
            image::open(PathBuf::from(path)).map(|image| (node_id.into(), image.to_rgba8()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let report = assess_direction_grid_appearance(&canonical, &nodes, equipment);
    let canonical_signature = image_signature(&canonical);
    let canonical_body = character_body_bbox(&canonical).unwrap();
    let canonical_outside = canonical
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            pixel[3] > 48
                && *y as f32 >= canonical_body.top
                && *y as f32 <= canonical_body.bottom_y
                && (*x as f32 + 2.0 < canonical_body.left || *x as f32 > canonical_body.right + 2.0)
        })
        .count();
    eprintln!(
        "canonical foreground_area={:.4} width={:.4} edge={:.4} detached={:.4} outside={} body=({:.1},{:.1})-({:.1},{:.1})",
        canonical_signature.foreground_area_ratio,
        canonical_signature.foreground_width_ratio,
        canonical_signature.edge_density,
        canonical_signature.detached_component_ratio,
        canonical_outside,
        canonical_body.left,
        canonical_body.top,
        canonical_body.right,
        canonical_body.bottom_y
    );
    eprintln!(
        "  canonical exterior components: {:?}",
        exterior_components(&canonical)
    );
    for (node_id, image) in &nodes {
        let signature = image_signature(image);
        let body = character_body_bbox(image).unwrap();
        let outside = image
            .enumerate_pixels()
            .filter(|(x, y, pixel)| {
                pixel[3] > 48
                    && *y as f32 >= body.top
                    && *y as f32 <= body.bottom_y
                    && (*x as f32 + 2.0 < body.left || *x as f32 > body.right + 2.0)
            })
            .count();
        eprintln!(
            "{node_id} foreground_area={:.4} width={:.4} edge={:.4} detached={:.4} outside={} body=({:.1},{:.1})-({:.1},{:.1})",
            signature.foreground_area_ratio,
            signature.foreground_width_ratio,
            signature.edge_density,
            signature.detached_component_ratio,
            outside,
            body.left,
            body.top,
            body.right,
            body.bottom_y
        );
        eprintln!("  exterior components: {:?}", exterior_components(image));
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn exterior_components(image: &image::RgbaImage) -> Vec<(usize, u32, u32)> {
    let body = character_body_bbox(image).unwrap();
    let mut exterior = BTreeSet::new();
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 48
            && y as f32 >= body.top
            && y as f32 <= body.bottom_y
            && (x as f32 + 2.0 < body.left || x as f32 > body.right + 2.0)
        {
            exterior.insert((x, y));
        }
    }
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for start in exterior.iter().copied() {
        if !seen.insert(start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let (mut left, mut right, mut top, mut bottom) = (start.0, start.0, start.1, start.1);
        let mut area = 0;
        while let Some((x, y)) = queue.pop_front() {
            area += 1;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            for ny in y.saturating_sub(1)..=(y + 1).min(image.height() - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(image.width() - 1) {
                    if exterior.contains(&(nx, ny)) && seen.insert((nx, ny)) {
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
        output.push((area, right - left + 1, bottom - top + 1));
    }
    output.sort_by_key(|value| std::cmp::Reverse(value.0));
    output.truncate(12);
    output
}
