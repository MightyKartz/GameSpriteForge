use std::env;
use std::error::Error;
use std::path::PathBuf;

use forge_core::motion_video::{
    create_idle_motion_driver_lock, CreateIdleMotionDriverLockRequestV1,
    IdleMotionDriverSourceKindV1,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let output = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_idle_motion_driver_lock <output> <id> <direction> <identity-anchor> <duration-ms> <cycles-min> <max-torso-ratio> <max-limb-ratio>",
    )?;
    let id = arguments
        .next()
        .ok_or("missing id")?
        .to_string_lossy()
        .into_owned();
    let direction = arguments
        .next()
        .ok_or("missing direction")?
        .to_string_lossy()
        .into_owned();
    let identity_anchor = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("missing identity anchor")?;
    let duration_ms = arguments
        .next()
        .ok_or("missing duration")?
        .to_string_lossy()
        .parse::<u64>()?;
    let cycles_minimum = arguments
        .next()
        .ok_or("missing cycles minimum")?
        .to_string_lossy()
        .parse::<usize>()?;
    let maximum_torso_ratio = arguments
        .next()
        .ok_or("missing maximum torso ratio")?
        .to_string_lossy()
        .parse::<f32>()?;
    let maximum_limb_ratio = arguments
        .next()
        .ok_or("missing maximum limb ratio")?
        .to_string_lossy()
        .parse::<f32>()?;
    if arguments.next().is_some() {
        return Err("too many arguments".into());
    }
    let lock = create_idle_motion_driver_lock(&CreateIdleMotionDriverLockRequestV1 {
        output_path: output,
        id,
        action: format!("idle_{direction}"),
        direction,
        source_kind: IdleMotionDriverSourceKindV1::IdentityAnchor,
        identity_anchor_path: identity_anchor,
        driver_source_path: None,
        requested_duration_ms: duration_ms,
        expected_cycles_minimum: cycles_minimum,
        maximum_torso_vertical_excursion_ratio: maximum_torso_ratio,
        maximum_limb_displacement_ratio: maximum_limb_ratio,
        background_hex: "#FF00FF".into(),
        aspect_ratio: "1:1".into(),
    })?;
    println!("{}", serde_json::to_string_pretty(&lock)?);
    Ok(())
}
