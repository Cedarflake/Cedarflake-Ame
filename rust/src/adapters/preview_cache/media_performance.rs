use std::fs::{self, File};
use std::time::Instant;

use image::GenericImageView;
use tempfile::tempdir;

use crate::adapters::local_files::canonical_source_root_path;
use crate::domain::DiscoveredFile;
use crate::media_fixtures::{
    ALL_MEDIA_FORMATS, MediaFixtureFormat, QUADRANT_COLORS, encode_rgb_quadrants,
};
use crate::ports::PreviewStore;

use super::LocalPreviewStore;

const CACHE_BUDGET_BYTES: u64 = 8 * 1024 * 1024;

#[test]
#[ignore = "explicit bounded synthetic media performance evidence"]
fn benchmark_supported_media_preview_cold_and_warm() {
    assert_eq!(ALL_MEDIA_FORMATS.len(), 7);
    for format in ALL_MEDIA_FORMATS {
        measure_format(format);
    }
    println!("AME_MEDIA_BENCHMARK formats=7 cold_generated=7 warm_reused=7 sources_unchanged=7");
}

fn measure_format(format: MediaFixtureFormat) {
    let (name, width, height, bucket, expected_preview) = match format {
        MediaFixtureFormat::Jpeg => ("jpeg", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::Png => ("png", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::WebP => ("webp", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::Gif => ("gif", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::Bmp => ("bmp", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::Tiff => ("tiff", 3000, 2000, 512, (512, 341)),
        MediaFixtureFormat::Ico => ("ico", 256, 256, 256, (256, 256)),
    };
    let storage = tempdir().expect("isolated media benchmark storage");
    let source_root = storage.path().join("source");
    fs::create_dir(&source_root).expect("isolated source directory");
    let fixture_started = Instant::now();
    let source_before = encode_rgb_quadrants(format, width, height).expect("encode fixture");
    let relative_path = format!("fixture.{}", format.extension());
    let source_path = source_root.join(&relative_path);
    fs::write(&source_path, &source_before).expect("write generated fixture");
    let fixture_ms = fixture_started.elapsed().as_millis();
    let metadata_before = source_path.metadata().expect("source metadata");
    assert!(!source_before.is_empty());
    assert_eq!(
        image::guess_format(&source_before).expect("encoded fixture signature"),
        format.image_format()
    );
    let discovered = DiscoveredFile {
        source_root_path: canonical_source_root_path(&source_root)
            .expect("canonical fixture source")
            .to_string_lossy()
            .into_owned(),
        absolute_path: source_path.to_string_lossy().into_owned(),
        relative_path,
        file_size: metadata_before.len(),
        created_unix_ms: None,
        modified_unix_ms: 0,
        file_identity: None,
        source_revision: None,
        source_generation: 1,
        issues: Vec::new(),
    };
    let cache_root = storage.path().join("previews");
    let store = LocalPreviewStore::new(cache_root.clone(), CACHE_BUDGET_BYTES)
        .expect("bounded preview store");
    assert_eq!(store.used_bytes(), 0);
    assert_eq!(fs::read_dir(&cache_root).expect("empty cache").count(), 0);

    let cold_started = Instant::now();
    let source = File::open(&source_path).expect("open generated source");
    let generated = store
        .materialize(&discovered, &source, bucket, 0, 0, false)
        .expect("cold production materialization");
    assert!(generated.staged_path.is_some(), "{name} must be generated");
    assert!(generated.reserved_bytes > 0);
    let cold = store.commit(generated).expect("cold artifact commit");
    drop(source);
    let cold_us = cold_started.elapsed().as_micros();
    assert_eq!((cold.width, cold.height), (width, height), "{name}");
    assert_eq!(cold.size_bucket, bucket);
    assert_eq!((cold.encoded_width, cold.encoded_height), expected_preview);
    let preview_before = fs::read(&cold.path).expect("cold encoded bytes");
    assert_eq!(cold.byte_size, preview_before.len() as u64);
    assert!(cold.byte_size > 0 && cold.byte_size <= CACHE_BUDGET_BYTES);
    let decoded = image::load_from_memory_with_format(&preview_before, image::ImageFormat::Jpeg)
        .expect("decode actual preview artifact");
    assert_eq!(decoded.dimensions(), expected_preview);
    for (index, expected) in QUADRANT_COLORS.iter().enumerate() {
        let x = if index % 2 == 0 {
            decoded.width() / 4
        } else {
            decoded.width() * 3 / 4
        };
        let y = if index < 2 {
            decoded.height() / 4
        } else {
            decoded.height() * 3 / 4
        };
        let actual = decoded.get_pixel(x, y);
        for channel in 0..3 {
            assert!(
                actual[channel].abs_diff(expected[channel]) <= 32,
                "{name} pixels"
            );
        }
    }
    let cache_bytes = store.used_bytes();
    assert_eq!(cache_bytes, cold.byte_size);
    let artifact_modified = fs::metadata(&cold.path)
        .expect("artifact metadata")
        .modified()
        .expect("artifact modification time");

    let warm_started = Instant::now();
    let source = File::open(&source_path).expect("reopen generated source");
    let reused = store
        .materialize(&discovered, &source, bucket, cold.width, cold.height, false)
        .expect("warm production materialization");
    assert!(reused.staged_path.is_none(), "{name} must reuse the cache");
    assert_eq!(reused.reserved_bytes, 0);
    let warm = store.commit(reused).expect("warm artifact commit");
    drop(source);
    let warm_us = warm_started.elapsed().as_micros();
    assert_eq!(warm.path, cold.path);
    assert_eq!(warm.artifact_key, cold.artifact_key);
    assert_eq!((warm.width, warm.height), (width, height));
    assert_eq!((warm.encoded_width, warm.encoded_height), expected_preview);
    assert_eq!(warm.byte_size, cold.byte_size);
    assert_eq!(store.used_bytes(), cache_bytes);
    assert_eq!(
        fs::read_dir(&cache_root)
            .expect("settled cache entries")
            .count(),
        1
    );
    assert_eq!(
        fs::read(&warm.path).expect("warm encoded bytes"),
        preview_before
    );
    assert_eq!(
        fs::metadata(&warm.path)
            .expect("warm metadata")
            .modified()
            .expect("warm modification time"),
        artifact_modified
    );
    assert_eq!(
        fs::read(&source_path).expect("source after requests"),
        source_before
    );
    let metadata_after = source_path.metadata().expect("source after metadata");
    assert_eq!(metadata_after.len(), metadata_before.len());
    assert_eq!(
        metadata_after
            .modified()
            .expect("source after modification time"),
        metadata_before
            .modified()
            .expect("source before modification time")
    );
    assert_eq!(
        fs::read_dir(&source_root).expect("source entries").count(),
        1
    );
    println!(
        "AME_MEDIA_FORMAT format={name} source_width={width} source_height={height} \
         source_bytes={} fixture_ms={fixture_ms} cold_us={cold_us} warm_us={warm_us} \
         bucket={bucket} preview_width={} preview_height={} preview_bytes={} \
         cache_bytes={cache_bytes} source_unchanged=true cache_reused=true",
        metadata_before.len(),
        cold.encoded_width,
        cold.encoded_height,
        cold.byte_size
    );
    drop(store);
    storage
        .close()
        .expect("release only owned fixture and derived cache");
}
