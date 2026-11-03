use std::cell::Cell;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rust_lib_cedarflake_ame::classification::{
    ClassificationEncoderKind, ClassificationEncoderResidency, ResidencyClock,
};
use rust_lib_cedarflake_ame::classification_worker::{
    LocalClassificationEncoderFactory, prepared_image_length,
};
use serde_json::json;

use super::verification::{cosine, floats};

#[derive(Clone)]
struct Clock(Rc<Cell<Duration>>);

impl ResidencyClock for Clock {
    fn elapsed(&self) -> Duration {
        self.0.get()
    }
}

pub(super) fn run(arguments: &[OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let factory = LocalClassificationEncoderFactory {
        executable: arguments[0].clone().into(),
        model_directory: arguments[1].clone().into(),
        runtime_directory: arguments[2].clone().into(),
    };
    let clock = Clock(Rc::new(Cell::new(Duration::ZERO)));
    let mut owner =
        ClassificationEncoderResidency::new(factory, clock.clone(), Duration::from_secs(60));
    let fixtures = Path::new(&arguments[3]);
    let pixels = floats(&fixtures.join("pixels.f32"), 32 * prepared_image_length())?;
    let image_reference = floats(&fixtures.join("image-t1-b1.bin"), 32 * 768)?;
    let text_reference = floats(&fixtures.join("text-t1-b1.bin"), 24 * 768)?;
    let prompt_path = fixtures.join("prompts.json");
    if fs::metadata(&prompt_path)?.len() > 128 * 1024 {
        return Err("Cached descriptions exceed their fixture bound".into());
    }
    let prompts: Vec<String> = serde_json::from_slice(&fs::read(prompt_path)?)?;
    if prompts.len() != 24 {
        return Err("Expected the retained description fixture".into());
    }
    let cancelled = AtomicBool::new(false);
    let mut minimum_cosine = 1.0_f64;
    for (prompt, reference) in prompts.iter().zip(text_reference.chunks_exact(768)).take(2) {
        let feature = owner.encode_description(prompt, &cancelled)?;
        minimum_cosine = minimum_cosine.min(cosine(feature.embedding.values(), reference));
    }
    owner.finish_descriptions()?;
    assert_eq!(owner.resident_kind(), None);
    println!("{}", json!({"stage":"text_released"}));

    for (input, reference) in pixels
        .chunks_exact(prepared_image_length())
        .zip(image_reference.chunks_exact(768))
        .take(2)
    {
        let feature = owner.encode_prepared_image(input, &cancelled)?;
        minimum_cosine = minimum_cosine.min(cosine(feature.embedding.values(), reference));
    }
    clock.0.set(Duration::from_millis(59_999));
    owner.retire_if_idle()?;
    assert_eq!(
        owner.resident_kind(),
        Some(ClassificationEncoderKind::Image)
    );
    clock.0.set(Duration::from_secs(60));
    owner.retire_if_idle()?;
    assert_eq!(owner.resident_kind(), None);
    println!("{}", json!({"stage":"idle_released"}));

    owner.encode_prepared_image(&pixels[..prepared_image_length()], &cancelled)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        flag.store(true, Ordering::Release);
    });
    let started = Instant::now();
    let result = owner.encode_prepared_image(&pixels[..prepared_image_length()], &cancelled);
    timer.join().map_err(|_| "Cancellation timer failed")?;
    if result.as_ref().err().map(|error| error.code.as_str()) != Some("classification_cancelled")
        || owner.resident_kind().is_some()
    {
        return Err("Cancellation did not reject the feature and retire its process".into());
    }
    if minimum_cosine < 0.999999 {
        return Err("The residency owner changed the admitted feature result".into());
    }
    println!(
        "{}",
        json!({
            "stage":"residency_verified", "compared_images":2, "compared_descriptions":2,
            "minimum_cosine":minimum_cosine, "idle_clock":"controlled",
            "cancel_request_and_retirement_ms":started.elapsed().as_secs_f64() * 1000.0,
            "source_files_opened":0,
        })
    );
    Ok(())
}
