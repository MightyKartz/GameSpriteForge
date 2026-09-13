use forge_core::library::{
    self,
    delivery::VersionRef,
    intake::{self, SearchFilter},
    review::{self, ReviewRequest},
};
use image::{Rgba, RgbaImage};
use std::{fs, path::Path};

fn media(root: &Path) {
    fs::create_dir_all(root).unwrap();
    RgbaImage::from_pixel(8, 8, Rgba([200, 130, 40, 255]))
        .save(root.join("sprite.png"))
        .unwrap();
    let mut encoder = gif::Encoder::new(
        fs::File::create(root.join("motion.gif")).unwrap(),
        2,
        2,
        &[],
    )
    .unwrap();
    encoder.set_repeat(gif::Repeat::Infinite).unwrap();
    for (delay, color) in [(7, [255, 20, 0, 255]), (19, [0, 190, 90, 255])] {
        let mut pixels = color.repeat(4);
        let mut frame = gif::Frame::from_rgba_speed(2, 2, &mut pixels, 10);
        frame.delay = delay;
        encoder.write_frame(&frame).unwrap();
    }
    let pcm = vec![0u8; 1600];
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + pcm.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((pcm.len() as u32).to_le_bytes());
    wav.extend(pcm);
    fs::write(root.join("tone.wav"), wav).unwrap();
}

#[test]
fn offline_preview_escapes_metadata_and_preserves_gif_timing_and_wav_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let sources = temp.path().join("media");
    media(&sources);
    let library = temp.path().join("library");
    library::initialize(&library, "Preview").unwrap();
    let mut scan = intake::scan(&sources).unwrap();
    for item in &mut scan.batch.items {
        item.name = "<script>alert('metadata')</script>".into();
    }
    let results = intake::register(&library, &scan.batch).unwrap();
    let references = results
        .iter()
        .map(|r| VersionRef {
            asset_id: r.asset_id.clone(),
            revision: r.revision.clone(),
        })
        .collect::<Vec<_>>();
    let head = fs::read(library.join(".forge/catalog.json")).unwrap();
    let output = temp.path().join("preview");
    let report = library::preview::create(&library, &references, &output).unwrap();
    assert_eq!(report.media_files, 3);
    let html = fs::read_to_string(report.index_path).unwrap();
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("<audio controls"));
    assert!(html.contains("Content-Security-Policy"));
    let mut gif_path = None;
    let mut wav_path = None;
    for directory in fs::read_dir(output.join("media")).unwrap() {
        for file in fs::read_dir(directory.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            match path.extension().unwrap().to_str().unwrap() {
                "gif" => gif_path = Some(path),
                "wav" => wav_path = Some(path),
                _ => (),
            }
        }
    }
    let mut decoder = gif::DecodeOptions::new()
        .read_info(fs::File::open(gif_path.unwrap()).unwrap())
        .unwrap();
    let mut delays = vec![];
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        delays.push(frame.delay);
    }
    assert_eq!(delays, [7, 19]);
    assert_eq!(
        fs::read(wav_path.unwrap()).unwrap(),
        fs::read(sources.join("tone.wav")).unwrap()
    );
    assert_eq!(head, fs::read(library.join(".forge/catalog.json")).unwrap());
    assert!(library::preview::create(&library, &references, &output).is_err());
    fs::remove_dir_all(sources).unwrap();
    let unavailable =
        library::preview::create(&library, &references, &temp.path().join("missing-preview"))
            .unwrap();
    assert_eq!(unavailable.issues.len(), 3);
    assert_eq!(unavailable.media_files, 0);
}

#[test]
fn review_evidence_is_retained_and_new_versions_have_no_inherited_approval() {
    let temp = tempfile::tempdir().unwrap();
    let sources = temp.path().join("media");
    media(&sources);
    let library = temp.path().join("library");
    library::initialize(&library, "Review").unwrap();
    let mut scan = intake::scan(&sources).unwrap();
    scan.batch.items.retain(|i| i.kind == "image");
    let result = intake::register(&library, &scan.batch).unwrap();
    let reference = VersionRef {
        asset_id: result[0].asset_id.clone(),
        revision: result[0].revision.clone(),
    };
    let evidence = temp.path().join("review-notes.txt");
    fs::write(&evidence, "Original human review notes <img onerror='x'>").unwrap();
    let request = ReviewRequest {
        reference: reference.clone(),
        domain: "visual".into(),
        verdict: "approved".into(),
        statement: "<script>untrusted note</script>".into(),
        reviewer: "Example reviewer".into(),
        evidence: evidence.clone(),
    };
    let record = review::record(&library, &request).unwrap();
    fs::remove_file(evidence).unwrap();
    assert!(library.join(record.evidence_path).is_file());
    let annotated = review::annotate(
        &library,
        &reference.asset_id,
        Some("Reviewed sprite".into()),
        Some(vec!["battle".into()]),
    )
    .unwrap();
    assert_eq!(annotated.tags, ["battle"]);
    RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]))
        .save(&scan.batch.items[0].path)
        .unwrap();
    scan.batch.items[0].expected_content = intake::content_at(&scan.batch.items[0].path).unwrap();
    scan.batch.items[0].new_revision = true;
    intake::register(&library, &scan.batch).unwrap();
    let catalog = library::read_catalog(&library).unwrap();
    let asset = library::read_asset(&library, &catalog, &reference.asset_id).unwrap();
    assert_eq!(
        review::read_reviews(&library, &asset, &reference.revision)
            .unwrap()
            .len(),
        1
    );
    assert!(
        review::read_reviews(&library, &asset, asset.revisions.last().unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(asset.selected_revision.is_none());
    let hits = intake::search(
        &library,
        &SearchFilter {
            review_domain: Some("visual".into()),
            review_verdict: Some("approved".into()),
            limit: 20,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(hits.total, 1);
    assert_eq!(hits.items[0].revision, reference.revision);
    let references = asset
        .revisions
        .iter()
        .map(|revision| VersionRef {
            asset_id: reference.asset_id.clone(),
            revision: revision.clone(),
        })
        .collect::<Vec<_>>();
    let report =
        library::preview::create(&library, &references, &temp.path().join("comparison")).unwrap();
    let html = fs::read_to_string(report.index_path).unwrap();
    assert!(html.contains("visual: approved") && html.contains("visual: unknown"));
    assert!(!html.contains("<script>untrusted"));
}

#[test]
fn preview_reports_missing_changed_and_nonfile_evidence_without_panicking() {
    for state in [
        "missing",
        "empty_directory",
        "single_file_directory",
        "changed",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let source = temp.path().join("source.bin");
        fs::write(&source, "source").unwrap();
        library::initialize(&root, "Evidence types").unwrap();
        let batch = intake::scan(&source).unwrap().batch;
        let item = &intake::register(&root, &batch).unwrap()[0];
        let reference = VersionRef {
            asset_id: item.asset_id.clone(),
            revision: item.revision.clone(),
        };
        let evidence = temp.path().join("notes.txt");
        fs::write(&evidence, "original notes").unwrap();
        let record = review::record(
            &root,
            &ReviewRequest {
                reference: reference.clone(),
                domain: "visual".into(),
                verdict: "unknown".into(),
                statement: "Fixture".into(),
                reviewer: "Fixture".into(),
                evidence,
            },
        )
        .unwrap();
        let stored = root.join(record.evidence_path);
        fs::remove_file(&stored).unwrap();
        match state {
            "missing" => (),
            "empty_directory" => fs::create_dir(&stored).unwrap(),
            "single_file_directory" => {
                fs::create_dir(&stored).unwrap();
                fs::write(stored.join("matching.bin"), "original notes").unwrap();
            }
            "changed" => fs::write(&stored, "changed notes").unwrap(),
            _ => unreachable!(),
        }
        let head = fs::read(root.join(".forge/catalog.json")).unwrap();
        let report =
            library::preview::create(&root, &[reference], &temp.path().join("preview")).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("review evidence")),
            "{state}"
        );
        let html = fs::read_to_string(report.index_path).unwrap();
        assert!(html.contains("unavailable or changed"), "{state}");
        assert_eq!(fs::read(root.join(".forge/catalog.json")).unwrap(), head);
    }
}
