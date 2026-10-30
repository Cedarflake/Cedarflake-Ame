use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;
use tokenizers::processors::template::TemplateProcessing;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::domain::classification::Embedding;

use super::artifacts::ArtifactSet;
use super::dll_search::RuntimeDllDirectory;
use super::manifest::{EMBEDDING_VALUES, IMAGE_VALUES, TOKENS};
use super::{EncoderKind, RuntimeError};

pub(super) struct Encoder {
    session: Session,
    tokenizer: Option<Tokenizer>,
    kind: EncoderKind,
    _artifacts: ArtifactSet,
    _dll_directory: RuntimeDllDirectory,
}

impl Encoder {
    pub fn load(artifacts: ArtifactSet, kind: EncoderKind) -> Result<Self, RuntimeError> {
        if std::env::var("ORT_DISABLE_TELEMETRY").as_deref() != Ok("1")
            || std::env::var("TOKENIZERS_PARALLELISM").as_deref() != Ok("false")
        {
            return Err(RuntimeError::Configuration);
        }
        let dll_directory = RuntimeDllDirectory::install(
            artifacts
                .runtime
                .parent()
                .ok_or(RuntimeError::Configuration)?,
        )?;
        let environment = ort::init_from(&artifacts.runtime)
            .map_err(|_| RuntimeError::RuntimeUnavailable)?
            .with_name("ame-classification")
            .with_telemetry(false)
            .build()
            .map_err(|_| RuntimeError::RuntimeUnavailable)?;
        let level = match kind {
            EncoderKind::Image => GraphOptimizationLevel::Level1,
            EncoderKind::Text => GraphOptimizationLevel::Level3,
        };
        let session = (|| -> ort::Result<Session> {
            Ok(Session::builder(&environment)?
                .with_intra_threads(1)?
                .with_inter_threads(1)?
                .with_parallel_execution(false)?
                .with_optimization_level(level)?
                .with_config_entry("session.intra_op.allow_spinning", "0")?
                .with_config_entry("session.inter_op.allow_spinning", "0")?
                .commit_from_file(&artifacts.graph)?)
        })()
        .map_err(|_| RuntimeError::RuntimeUnavailable)?;
        let tokenizer = artifacts
            .tokenizer
            .as_deref()
            .map(load_tokenizer)
            .transpose()?;
        Ok(Self {
            session,
            tokenizer,
            kind,
            _artifacts: artifacts,
            _dll_directory: dll_directory,
        })
    }

    pub fn image(&mut self, pixels: Vec<f32>) -> Result<Embedding, RuntimeError> {
        if self.kind != EncoderKind::Image
            || pixels.len() != IMAGE_VALUES
            || pixels.iter().any(|value| !(-1.0..=1.0).contains(value))
        {
            return Err(RuntimeError::InvalidInput);
        }
        let input =
            Tensor::from_array(([1, 3, 224, 224], pixels)).map_err(|_| RuntimeError::Inference)?;
        let output = self
            .session
            .run(ort::inputs!["pixel_values" => input])
            .map_err(|_| RuntimeError::Inference)?;
        extract_embedding(output.get("pooler_output").ok_or(RuntimeError::Inference)?)
    }

    pub fn text(&mut self, description: &str) -> Result<Embedding, RuntimeError> {
        if self.kind != EncoderKind::Text || !valid_description(description) {
            return Err(RuntimeError::InvalidInput);
        }
        let tokenizer = self.tokenizer.as_ref().ok_or(RuntimeError::Configuration)?;
        let encoded = tokenizer
            .encode(description, true)
            .map_err(|_| RuntimeError::InvalidInput)?;
        let ids: Vec<_> = encoded.get_ids().iter().map(|id| i64::from(*id)).collect();
        if ids.len() != TOKENS {
            return Err(RuntimeError::Inference);
        }
        let input = Tensor::from_array(([1, TOKENS], ids)).map_err(|_| RuntimeError::Inference)?;
        let output = self
            .session
            .run(ort::inputs!["input_ids" => input])
            .map_err(|_| RuntimeError::Inference)?;
        extract_embedding(output.get("pooler_output").ok_or(RuntimeError::Inference)?)
    }
}

pub(super) fn valid_description(description: &str) -> bool {
    !description.is_empty()
        && description.trim() == description
        && description.chars().count() <= 1024
        && !description.chars().any(char::is_control)
}

fn extract_embedding(output: &ort::value::DynValue) -> Result<Embedding, RuntimeError> {
    let (shape, values) = output
        .try_extract_tensor::<f32>()
        .map_err(|_| RuntimeError::Inference)?;
    if shape.as_ref() != [1, EMBEDDING_VALUES as i64] {
        return Err(RuntimeError::Inference);
    }
    Embedding::normalized(values.to_vec()).map_err(|_| RuntimeError::Inference)
}

fn load_tokenizer(path: &std::path::Path) -> Result<Tokenizer, RuntimeError> {
    let load = || -> tokenizers::Result<Tokenizer> {
        let mut tokenizer = Tokenizer::from_file(path)?;
        let eos = tokenizer.token_to_id("<eos>").ok_or("Missing EOS token")?;
        let pad = tokenizer
            .token_to_id("<pad>")
            .ok_or("Missing padding token")?;
        let processor = TemplateProcessing::builder()
            .try_single("$A <eos>")?
            .special_tokens(vec![("<eos>", eos)])
            .build()?;
        tokenizer.with_post_processor(Some(processor));
        tokenizer.with_truncation(Some(TruncationParams {
            max_length: TOKENS,
            ..Default::default()
        }))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(TOKENS),
            pad_id: pad,
            pad_token: "<pad>".into(),
            ..Default::default()
        }));
        Ok(tokenizer)
    };
    load().map_err(|_| RuntimeError::RuntimeUnavailable)
}
