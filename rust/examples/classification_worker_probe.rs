#[cfg(all(windows, feature = "classification-worker"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    verification::run()
}

#[cfg(not(all(windows, feature = "classification-worker")))]
fn main() {
    eprintln!("This probe requires Windows and the classification-worker feature");
    std::process::exit(1);
}

#[cfg(all(windows, feature = "classification-worker"))]
mod verification {
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use rust_lib_cedarflake_ame::classification_worker::{
        EncoderKind, RuntimeError, WorkerConfiguration, WorkerSession, prepared_image_length,
    };
    use serde_json::json;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    pub fn run() -> Result<()> {
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if arguments.len() != 4 {
            return Err("Expected worker executable, model directory, runtime directory and cached fixture directory".into());
        }
        let configuration = |encoder| WorkerConfiguration {
            executable: arguments[0].clone().into(),
            model_directory: arguments[1].clone().into(),
            runtime_directory: arguments[2].clone().into(),
            encoder,
        };
        let fixtures = Path::new(&arguments[3]);
        let pixels = floats(&fixtures.join("pixels.f32"), 32 * prepared_image_length())?;
        let image_reference = floats(&fixtures.join("image-t1-b1.bin"), 32 * 768)?;
        let text_reference = floats(&fixtures.join("text-t1-b1.bin"), 24 * 768)?;
        let prompts_path = fixtures.join("prompts.json");
        if fs::metadata(&prompts_path)?.len() > 128 * 1024 {
            return Err("Cached descriptions exceed the fixture bound".into());
        }
        let prompts: Vec<String> = serde_json::from_slice(&fs::read(prompts_path)?)?;
        if prompts.len() != 24 {
            return Err("Expected the retained 24-description fixture".into());
        }
        let cancelled = AtomicBool::new(false);
        let started = Instant::now();
        let mut image = WorkerSession::start(configuration(EncoderKind::Image), &cancelled)?;
        let image_load_ms = started.elapsed().as_secs_f64() * 1000.0;
        println!("{}", json!({"stage":"image_ready"}));
        assert!(matches!(
            image.encode_prepared_image(&[0.0], &cancelled),
            Err(RuntimeError::InvalidInput)
        ));
        let mut image_cosine = 1.0_f64;
        for (input, expected) in pixels
            .chunks_exact(prepared_image_length())
            .zip(image_reference.chunks_exact(768))
        {
            let feature = image.encode_prepared_image(input, &cancelled)?;
            image_cosine = image_cosine.min(cosine(feature.embedding.values(), expected));
        }
        let started = Instant::now();
        image.shutdown()?;
        let image_shutdown_ms = started.elapsed().as_secs_f64() * 1000.0;

        let started = Instant::now();
        let mut text = WorkerSession::start(configuration(EncoderKind::Text), &cancelled)?;
        let text_load_ms = started.elapsed().as_secs_f64() * 1000.0;
        println!("{}", json!({"stage":"text_ready"}));
        assert!(matches!(
            text.encode_description("", &cancelled),
            Err(RuntimeError::InvalidInput)
        ));
        let mut text_cosine = 1.0_f64;
        for (prompt, expected) in prompts.iter().zip(text_reference.chunks_exact(768)) {
            let feature = text.encode_description(prompt, &cancelled)?;
            text_cosine = text_cosine.min(cosine(feature.embedding.values(), expected));
        }
        text.shutdown()?;

        let mut image = WorkerSession::start(configuration(EncoderKind::Image), &cancelled)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let timer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            flag.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let result = image.encode_prepared_image(&pixels[..prepared_image_length()], &cancelled);
        timer.join().map_err(|_| "Cancellation timer failed")?;
        if !matches!(result, Err(RuntimeError::Cancelled)) || !image.is_retired() {
            return Err("Cancellation did not retire the native worker".into());
        }
        if image_cosine < 0.999999 || text_cosine < 0.999999 {
            return Err("The supervised worker differs from the admitted encoder fixture".into());
        }
        println!(
            "{}",
            json!({
                "stage":"verified", "images":32, "descriptions":24,
                "image_minimum_cosine":image_cosine, "text_minimum_cosine":text_cosine,
                "image_load_ms":image_load_ms, "text_load_ms":text_load_ms,
                "image_shutdown_ms":image_shutdown_ms,
                "cancel_request_and_retirement_ms":started.elapsed().as_secs_f64() * 1000.0,
                "source_files_opened":0,
            })
        );
        Ok(())
    }

    fn floats(path: &Path, expected: usize) -> Result<Vec<f32>> {
        if fs::metadata(path)?.len() != expected as u64 * 4 {
            return Err("Cached tensor length does not match the retained fixture".into());
        }
        Ok(fs::read(path)?
            .chunks_exact(4)
            .map(|part| f32::from_le_bytes(part.try_into().unwrap()))
            .collect())
    }

    fn cosine(left: &[f32], right: &[f32]) -> f64 {
        let dot: f64 = left
            .iter()
            .zip(right)
            .map(|(a, b)| f64::from(*a) * f64::from(*b))
            .sum();
        let norm = |values: &[f32]| {
            values
                .iter()
                .map(|value| f64::from(*value).powi(2))
                .sum::<f64>()
                .sqrt()
        };
        dot / (norm(left) * norm(right))
    }
}
