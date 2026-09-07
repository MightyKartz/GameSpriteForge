use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::{imageops::FilterType, DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const IDENTITY_METRIC_PROFILE: &str = "identity-metric@2.0.0";
pub const IDENTITY_CALIBRATION_PROFILE: &str = "identity-calibration@1.0.0";

#[derive(Debug, Error)]
pub enum IdentityMetricError {
    #[error("identity metric input is invalid: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityLabelV1 {
    Pass,
    Gray,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityCalibrationPairV1 {
    pub id: String,
    pub left: PathBuf,
    pub right: PathBuf,
    pub label: IdentityLabelV1,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityCalibrationSetV1 {
    pub schema_version: String,
    pub profile: String,
    pub created_at: String,
    pub method: String,
    pub pairs: Vec<IdentityCalibrationPairV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentitySignalScoresV1 {
    pub hog_similarity: f32,
    pub regional_palette_emd: f32,
    pub silhouette_iou: f32,
    pub perceptual_similarity: f32,
    pub occupancy_similarity: f32,
    pub composite: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMetricReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub left_sha256: String,
    pub right_sha256: String,
    pub weights: IdentityMetricWeightsV1,
    pub scores: IdentitySignalScoresV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMetricWeightsV1 {
    pub hog_similarity: f32,
    pub regional_palette_emd: f32,
    pub silhouette_iou: f32,
    pub perceptual_similarity: f32,
    pub occupancy_similarity: f32,
}

impl Default for IdentityMetricWeightsV1 {
    fn default() -> Self {
        Self {
            // The initial real-run calibration set showed regional palette
            // preservation was the only signal that separated accepted
            // direction edits from human-rejected identity drift. Silhouette
            // IoU is retained in the report but receives zero weight because
            // legitimate front→side direction changes score lower than
            // human-rejected front→back structure loss.
            hog_similarity: 0.05,
            regional_palette_emd: 0.80,
            silhouette_iou: 0.0,
            perceptual_similarity: 0.05,
            occupancy_similarity: 0.10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityPairEvaluationV1 {
    pub id: String,
    pub label: IdentityLabelV1,
    pub phash_similarity: f32,
    pub scores: IdentitySignalScoresV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityConfusionMatrixV1 {
    pub true_positive: u32,
    pub false_positive: u32,
    pub true_negative: u32,
    pub false_negative: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMetricEvaluationV1 {
    pub metric: String,
    pub pair_count: u32,
    pub pass_mean: f32,
    pub fail_mean: f32,
    pub separation: f32,
    pub roc_auc: f32,
    pub best_threshold: f32,
    pub confusion: IdentityConfusionMatrixV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityEvaluationReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub calibration: IdentityCalibrationSummaryV1,
    pub baseline_phash: IdentityMetricEvaluationV1,
    pub ablation: Vec<IdentityMetricEvaluationV1>,
    pub composite: IdentityMetricEvaluationV1,
    pub pairs: Vec<IdentityPairEvaluationV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityCalibrationSummaryV1 {
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
    pub pair_count: u32,
    pub pass_count: u32,
    pub gray_count: u32,
    pub fail_count: u32,
}

pub fn evaluate_identity_metric(
    candidate: &RgbaImage,
    reference: &RgbaImage,
) -> IdentityMetricReportV1 {
    let weights = IdentityMetricWeightsV1::default();
    evaluate_identity_metric_with_weights(candidate, reference, weights)
}

pub fn evaluate_identity_metric_with_weights(
    candidate: &RgbaImage,
    reference: &RgbaImage,
    weights: IdentityMetricWeightsV1,
) -> IdentityMetricReportV1 {
    let candidate = normalize_for_analysis(candidate);
    let reference = normalize_for_analysis(reference);
    let hog_similarity = hog_similarity(&candidate, &reference);
    let regional_palette_emd = regional_palette_similarity(&candidate, &reference, 3);
    let silhouette_iou = silhouette_iou(&candidate, &reference);
    let perceptual_similarity = perceptual_similarity(&candidate, &reference);
    let occupancy_similarity = occupancy_similarity(&candidate, &reference);
    let composite = (hog_similarity * weights.hog_similarity
        + regional_palette_emd * weights.regional_palette_emd
        + silhouette_iou * weights.silhouette_iou
        + perceptual_similarity * weights.perceptual_similarity
        + occupancy_similarity * weights.occupancy_similarity)
        .clamp(0.0, 1.0);
    IdentityMetricReportV1 {
        schema_version: "1".into(),
        profile: IDENTITY_METRIC_PROFILE.into(),
        left_sha256: rgba_sha256(&candidate),
        right_sha256: rgba_sha256(&reference),
        weights,
        scores: IdentitySignalScoresV1 {
            hog_similarity,
            regional_palette_emd,
            silhouette_iou,
            perceptual_similarity,
            occupancy_similarity,
            composite,
        },
    }
}

pub fn evaluate_calibration_manifest(
    manifest_path: &Path,
) -> Result<IdentityEvaluationReportV1, IdentityMetricError> {
    let manifest_bytes = fs::read(manifest_path)?;
    let manifest: IdentityCalibrationSetV1 = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_version != "1" || manifest.profile != IDENTITY_CALIBRATION_PROFILE {
        return Err(IdentityMetricError::Invalid(
            "unsupported identity calibration manifest".into(),
        ));
    }
    let root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut pairs = Vec::new();
    for pair in &manifest.pairs {
        let left = image::open(root.join(&pair.left))?.to_rgba8();
        let right = image::open(root.join(&pair.right))?.to_rgba8();
        let report = evaluate_identity_metric(&left, &right);
        pairs.push(IdentityPairEvaluationV1 {
            id: pair.id.clone(),
            label: pair.label,
            phash_similarity: report.scores.perceptual_similarity,
            scores: report.scores,
        });
    }
    let metrics = [
        "phash",
        "hog",
        "regional_palette_emd",
        "silhouette_iou",
        "occupancy",
        "composite",
    ];
    let mut evaluations = metrics
        .iter()
        .map(|name| evaluate_scores(name, &pairs, |pair| score_by_metric(pair, name)))
        .collect::<Vec<_>>();
    let baseline = evaluations.remove(0);
    let composite = evaluations
        .pop()
        .expect("composite evaluation is always present");
    let pass_count = pairs
        .iter()
        .filter(|pair| pair.label == IdentityLabelV1::Pass)
        .count() as u32;
    let gray_count = pairs
        .iter()
        .filter(|pair| pair.label == IdentityLabelV1::Gray)
        .count() as u32;
    let fail_count = pairs
        .iter()
        .filter(|pair| pair.label == IdentityLabelV1::Fail)
        .count() as u32;
    Ok(IdentityEvaluationReportV1 {
        schema_version: "1".into(),
        profile: IDENTITY_METRIC_PROFILE.into(),
        calibration: IdentityCalibrationSummaryV1 {
            manifest_path: manifest_path.to_path_buf(),
            manifest_sha256: format!("{:x}", Sha256::digest(&manifest_bytes)),
            pair_count: pairs.len() as u32,
            pass_count,
            gray_count,
            fail_count,
        },
        baseline_phash: baseline,
        ablation: evaluations,
        composite,
        pairs,
    })
}

fn evaluate_scores(
    metric: &str,
    pairs: &[IdentityPairEvaluationV1],
    score: impl Fn(&IdentityPairEvaluationV1) -> f32,
) -> IdentityMetricEvaluationV1 {
    let mut pass = pairs
        .iter()
        .filter(|pair| pair.label == IdentityLabelV1::Pass)
        .map(&score)
        .collect::<Vec<_>>();
    let mut fail = pairs
        .iter()
        .filter(|pair| pair.label == IdentityLabelV1::Fail)
        .map(&score)
        .collect::<Vec<_>>();
    pass.sort_by(f32::total_cmp);
    fail.sort_by(f32::total_cmp);
    let pass_mean = mean(&pass);
    let fail_mean = mean(&fail);
    let roc_auc = roc_auc(&pass, &fail);
    let mut best_threshold = 0.0;
    let mut best_youden = f32::NEG_INFINITY;
    let mut best_confusion = IdentityConfusionMatrixV1 {
        true_positive: 0,
        false_positive: 0,
        true_negative: 0,
        false_negative: 0,
    };
    for index in 0..=100 {
        let threshold = index as f32 / 100.0;
        let confusion = IdentityConfusionMatrixV1 {
            true_positive: pass.iter().filter(|value| **value >= threshold).count() as u32,
            false_positive: fail.iter().filter(|value| **value >= threshold).count() as u32,
            true_negative: fail.iter().filter(|value| **value < threshold).count() as u32,
            false_negative: pass.iter().filter(|value| **value < threshold).count() as u32,
        };
        let true_positive_rate = confusion.true_positive as f32 / pass.len().max(1) as f32;
        let false_positive_rate = confusion.false_positive as f32 / fail.len().max(1) as f32;
        let youden = true_positive_rate - false_positive_rate;
        if youden > best_youden {
            best_youden = youden;
            best_threshold = threshold;
            best_confusion = confusion;
        }
    }
    IdentityMetricEvaluationV1 {
        metric: metric.into(),
        pair_count: (pass.len() + fail.len()) as u32,
        pass_mean,
        fail_mean,
        separation: pass_mean - fail_mean,
        roc_auc,
        best_threshold,
        confusion: best_confusion,
    }
}

fn score_by_metric(pair: &IdentityPairEvaluationV1, metric: &str) -> f32 {
    match metric {
        "phash" => pair.phash_similarity,
        "hog" => pair.scores.hog_similarity,
        "regional_palette_emd" => pair.scores.regional_palette_emd,
        "silhouette_iou" => pair.scores.silhouette_iou,
        "occupancy" => pair.scores.occupancy_similarity,
        "composite" => pair.scores.composite,
        _ => 0.0,
    }
}

fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len().max(1) as f32
}

fn roc_auc(pass: &[f32], fail: &[f32]) -> f32 {
    if pass.is_empty() || fail.is_empty() {
        return 0.0;
    }
    let mut wins = 0.0;
    for pass_value in pass {
        for fail_value in fail {
            wins += if pass_value > fail_value {
                1.0
            } else if pass_value == fail_value {
                0.5
            } else {
                0.0
            };
        }
    }
    wins / (pass.len() * fail.len()) as f32
}

fn normalize_for_analysis(image: &RgbaImage) -> RgbaImage {
    DynamicImage::ImageRgba8(image.clone())
        .resize_exact(64, 64, FilterType::Triangle)
        .to_rgba8()
}

fn hog_similarity(left: &RgbaImage, right: &RgbaImage) -> f32 {
    let left = hog(left);
    let right = hog(right);
    cosine(&left, &right)
}

fn hog(image: &RgbaImage) -> [f32; 8] {
    let mut bins = [0.0_f32; 8];
    for y in 1..image.height().saturating_sub(1) {
        for x in 1..image.width().saturating_sub(1) {
            let alpha = image.get_pixel(x, y)[3];
            if alpha <= 16 {
                continue;
            }
            let gx = luma(image, x + 1, y) as f32 - luma(image, x - 1, y) as f32;
            let gy = luma(image, x, y + 1) as f32 - luma(image, x, y - 1) as f32;
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude < 8.0 {
                continue;
            }
            let angle = gy.atan2(gx).rem_euclid(std::f32::consts::TAU);
            let bin = ((angle / std::f32::consts::TAU) * 8.0).floor() as usize % 8;
            bins[bin] += magnitude;
        }
    }
    normalize_histogram(&mut bins);
    bins
}

fn luma(image: &RgbaImage, x: u32, y: u32) -> u8 {
    let pixel = image.get_pixel(x, y);
    ((u32::from(pixel[0]) * 299 + u32::from(pixel[1]) * 587 + u32::from(pixel[2]) * 114) / 1000)
        as u8
}

fn normalize_histogram(histogram: &mut [f32; 8]) {
    let norm = histogram
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > f32::EPSILON {
        for value in histogram {
            *value /= norm;
        }
    }
}

fn regional_palette_similarity(left: &RgbaImage, right: &RgbaImage, cells: u32) -> f32 {
    let mut scores = Vec::new();
    for row in 0..cells {
        for column in 0..cells {
            let left_palette = region_palette(left, cells, column, row);
            let right_palette = region_palette(right, cells, column, row);
            if !left_palette.is_empty() && !right_palette.is_empty() {
                scores.push(palette_support_similarity(&left_palette, &right_palette));
            }
        }
    }
    mean(&scores)
}

fn region_palette(image: &RgbaImage, cells: u32, column: u32, row: u32) -> BTreeMap<[u8; 3], f32> {
    let left = image.width() * column / cells;
    let right = image.width() * (column + 1) / cells;
    let top = image.height() * row / cells;
    let bottom = image.height() * (row + 1) / cells;
    let mut counts = BTreeMap::<[u8; 3], u64>::new();
    let mut total = 0_u64;
    for y in top..bottom {
        for x in left..right {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 16 {
                *counts
                    .entry([pixel[0] / 16, pixel[1] / 16, pixel[2] / 16])
                    .or_default() += 1;
                total += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|(key, count)| {
            (
                [key[0] * 16 + 8, key[1] * 16 + 8, key[2] * 16 + 8],
                count as f32 / total.max(1) as f32,
            )
        })
        .collect()
}

fn palette_support_similarity(
    left: &BTreeMap<[u8; 3], f32>,
    right: &BTreeMap<[u8; 3], f32>,
) -> f32 {
    let left_support = palette_directional_support(left, right);
    let right_support = palette_directional_support(right, left);
    ((left_support + right_support) / 2.0).clamp(0.0, 1.0)
}

fn palette_directional_support(
    left: &BTreeMap<[u8; 3], f32>,
    right: &BTreeMap<[u8; 3], f32>,
) -> f32 {
    left.iter()
        .map(|(color, weight)| {
            let support = right
                .keys()
                .map(|candidate| {
                    let distance_squared = color
                        .iter()
                        .zip(candidate)
                        .map(|(left, right)| left.abs_diff(*right) as f32)
                        .map(|delta| delta * delta)
                        .sum::<f32>();
                    (-distance_squared / (2.0 * 32.0_f32.powi(2))).exp()
                })
                .fold(0.0, f32::max);
            weight * support
        })
        .sum()
}

fn silhouette_iou(left: &RgbaImage, right: &RgbaImage) -> f32 {
    let mut intersection = 0_u64;
    let mut union = 0_u64;
    for (left, right) in left.pixels().zip(right.pixels()) {
        let left = left[3] > 16;
        let right = right[3] > 16;
        intersection += u64::from(left && right);
        union += u64::from(left || right);
    }
    intersection as f32 / union.max(1) as f32
}

fn perceptual_similarity(left: &RgbaImage, right: &RgbaImage) -> f32 {
    let left = perceptual_hash(left);
    let right = perceptual_hash(right);
    1.0 - (left ^ right).count_ones() as f32 / 64.0
}

fn perceptual_hash(image: &RgbaImage) -> u64 {
    let gray = DynamicImage::ImageRgba8(image.clone())
        .resize_exact(9, 8, FilterType::Triangle)
        .to_luma8();
    let mut hash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            if gray.get_pixel(x, y)[0] > gray.get_pixel(x + 1, y)[0] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    hash
}

fn occupancy_similarity(left: &RgbaImage, right: &RgbaImage) -> f32 {
    let left = occupancy_profile(left);
    let right = occupancy_profile(right);
    let delta = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum::<f32>()
        / 16.0;
    (1.0 - delta).clamp(0.0, 1.0)
}

fn occupancy_profile(image: &RgbaImage) -> [f32; 16] {
    let mut profile = [0.0_f32; 16];
    for (index, slot) in profile.iter_mut().enumerate() {
        let top = image.height() as usize * index / 16;
        let bottom = image.height() as usize * (index + 1) / 16;
        let mut occupied = 0_u64;
        for y in top..bottom.max(top + 1).min(image.height() as usize) {
            for x in 0..image.width() {
                occupied += u64::from(image.get_pixel(x, y as u32)[3] > 16);
            }
        }
        *slot = occupied as f32
            / ((bottom.max(top + 1).min(image.height() as usize) - top).max(1)
                * image.width() as usize) as f32;
    }
    profile
}

fn cosine(left: &[f32; 8], right: &[f32; 8]) -> f32 {
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    dot.clamp(0.0, 1.0)
}

fn rgba_sha256(image: &RgbaImage) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image.width().to_le_bytes());
    hasher.update(image.height().to_le_bytes());
    hasher.update(image.as_raw());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn test_image(offset: i32, color: [u8; 3]) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 18..50 {
            for x in 18..50 {
                let shifted_x = x + offset;
                if !(0..64).contains(&shifted_x) {
                    continue;
                }
                image.put_pixel(
                    shifted_x as u32,
                    y,
                    Rgba([color[0], color[1], color[2], 255]),
                );
            }
        }
        image
    }

    #[test]
    fn identity_metric_scores_same_shape_higher_than_a_different_color_shape() {
        let reference = test_image(0, [40, 100, 60]);
        let same = test_image(1, [45, 105, 65]);
        let different = test_image(-20, [220, 40, 30]);
        let same_score = evaluate_identity_metric(&same, &reference).scores.composite;
        let different_score = evaluate_identity_metric(&different, &reference)
            .scores
            .composite;
        assert!(same_score > different_score);
    }

    #[test]
    fn identity_metric_report_contains_all_ablation_signals() {
        let reference = test_image(0, [40, 100, 60]);
        let candidate = test_image(2, [42, 102, 62]);
        let report = evaluate_identity_metric(&candidate, &reference);
        assert_eq!(report.profile, IDENTITY_METRIC_PROFILE);
        assert!(report.scores.hog_similarity.is_finite());
        assert!(report.scores.regional_palette_emd.is_finite());
        assert!(report.scores.silhouette_iou.is_finite());
        assert!(report.scores.perceptual_similarity.is_finite());
        assert!(report.scores.occupancy_similarity.is_finite());
    }
}
