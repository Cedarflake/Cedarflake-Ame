use crate::domain::classification::{ClassificationRevision, ModelCompatibility};

pub(super) const IMAGE_VALUES: usize = 3 * 224 * 224;
pub(super) const EMBEDDING_VALUES: usize = 768;
pub(super) const TOKENS: usize = 64;

pub(super) struct Artifact {
    pub name: &'static str,
    pub length: u64,
    pub digest: &'static str,
}

pub(super) const IMAGE: Artifact = Artifact {
    name: "vision_model_fp16.onnx",
    length: 186_039_516,
    digest: "a1959f7bd3993a607e48839f6d01e25b876fe76afda301b028b78eef68aabd95",
};
pub(super) const TEXT: Artifact = Artifact {
    name: "text_model_quantized.onnx",
    length: 283_438_275,
    digest: "3a0603d3a00c05a80a6ded4743c16aaac7b1e62cdcc7e362e7ce418659b96400",
};
pub(super) const TOKENIZER: Artifact = Artifact {
    name: "tokenizer.json",
    length: 34_363_039,
    digest: "cb9140fae3ac5122c972d37adf83e1248471a38147ad76f8215c8872c6fd8322",
};
pub(super) const RUNTIME: Artifact = Artifact {
    name: "onnxruntime.dll",
    length: 17_253_728,
    digest: "cb2fa518b18356617f6f9f460c012501da2819ec7e5c1d80bd8f5de20492db0c",
};
pub(super) const PROVIDER: Artifact = Artifact {
    name: "onnxruntime_providers_shared.dll",
    length: 21_816,
    digest: "8cea20be64259950cda8643af0ed42c2a424400fc3bb9bb29f9a35cee6b3419f",
};

pub(super) fn compatibility() -> ModelCompatibility {
    let space = ClassificationRevision::derive(
        "ame.classification.embedding-space.v1",
        [IMAGE.digest.as_bytes(), TEXT.digest.as_bytes()],
    );
    let execution = b"ort-2.0.0-rc.15;cpu-1.31.1;sequential;threads-1;batch-1;no-spinning";
    let image = ClassificationRevision::derive(
        "ame.classification.image-pipeline.v1",
        [
            IMAGE.digest.as_bytes(),
            RUNTIME.digest.as_bytes(),
            execution.as_slice(),
            b"first-frame;oriented-rgb;whole-bilinear-224;f32-nchw;mean-0.5;std-0.5;level-1;l2-f64",
        ],
    );
    let text = ClassificationRevision::derive(
        "ame.classification.text-pipeline.v1",
        [
            TEXT.digest.as_bytes(),
            TOKENIZER.digest.as_bytes(),
            RUNTIME.digest.as_bytes(),
            execution.as_slice(),
            b"tokenizers-0.23.2;single-description;eos;pad-64;truncate-64;level-3;l2-f64",
        ],
    );
    ModelCompatibility::new(space, image, text, EMBEDDING_VALUES)
        .expect("the admitted model has 768 dimensions")
}

pub(super) fn contract_revision() -> [u8; 32] {
    let model = compatibility();
    *ClassificationRevision::derive(
        "ame.classification.worker-contract.v1",
        [
            model.image_identity().embedding_space.bytes().as_slice(),
            model.image_identity().pipeline.bytes().as_slice(),
            model.text_identity().pipeline.bytes().as_slice(),
            &(EMBEDDING_VALUES as u32).to_le_bytes(),
        ],
    )
    .bytes()
}
