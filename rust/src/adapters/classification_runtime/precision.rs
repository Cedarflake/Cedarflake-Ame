use std::path::Path;

use half::f16;
use prost::Message;

use super::RuntimeError;

// Preserve the official protobuf names in reproducible generated output.
#[allow(dead_code, clippy::enum_variant_names)]
mod onnx;
use onnx::{GraphProto, ModelProto, TensorProto, type_proto};

const FLOAT: i32 = 1;
const FLOAT16: i32 = 10;

pub(super) fn image_graph(path: &Path) -> Result<Vec<u8>, RuntimeError> {
    let source = std::fs::read(path)?;
    if source.len() as u64 != super::manifest::IMAGE.length {
        return Err(RuntimeError::ArtifactMismatch);
    }
    let mut model = ModelProto::decode(source.as_slice()).map_err(|_| rejected())?;
    drop(source);
    if !model.functions.is_empty() || !model.training_info.is_empty() {
        return Err(rejected());
    }
    let graph = model.graph.as_mut().ok_or_else(rejected)?;
    if promote_graph(graph)? != 218 {
        return Err(rejected());
    }
    Ok(model.encode_to_vec())
}

fn promote_graph(graph: &mut GraphProto) -> Result<usize, RuntimeError> {
    if !graph.sparse_initializer.is_empty() {
        return Err(rejected());
    }
    let mut converted = 0;
    for tensor in &mut graph.initializer {
        converted += promote_tensor(tensor)?;
    }
    for value in graph
        .input
        .iter_mut()
        .chain(&mut graph.output)
        .chain(&mut graph.value_info)
    {
        if let Some(type_proto::Value::TensorType(tensor)) =
            value.r#type.as_mut().and_then(|value| value.value.as_mut())
            && tensor.elem_type == Some(FLOAT16)
        {
            tensor.elem_type = Some(FLOAT);
        }
    }
    for node in &mut graph.node {
        for attribute in &mut node.attribute {
            if node.op_type.as_deref() == Some("Cast")
                && attribute.name.as_deref() == Some("to")
                && attribute.i == Some(i64::from(FLOAT16))
            {
                attribute.i = Some(i64::from(FLOAT));
            }
            if let Some(tensor) = &mut attribute.t {
                converted += promote_tensor(tensor)?;
            }
            for tensor in &mut attribute.tensors {
                converted += promote_tensor(tensor)?;
            }
            // The pinned graph has no subgraphs or sparse constants.
            if attribute.g.is_some()
                || !attribute.graphs.is_empty()
                || attribute.sparse_tensor.is_some()
                || !attribute.sparse_tensors.is_empty()
            {
                return Err(rejected());
            }
        }
    }
    Ok(converted)
}

fn promote_tensor(tensor: &mut TensorProto) -> Result<usize, RuntimeError> {
    if !tensor.external_data.is_empty() || tensor.data_location.unwrap_or_default() != 0 {
        return Err(rejected());
    }
    if tensor.data_type != Some(FLOAT16) {
        return Ok(0);
    }
    let count = tensor
        .dims
        .iter()
        .try_fold(1_usize, |count, dimension| {
            count.checked_mul(usize::try_from(*dimension).ok()?)
        })
        .ok_or_else(rejected)?;
    let raw = tensor.raw_data.take().ok_or_else(rejected)?;
    if count > 128 * 1024 * 1024
        || count.checked_mul(2) != Some(raw.len())
        || !tensor.int32_data.is_empty()
        || !tensor.float_data.is_empty()
    {
        return Err(rejected());
    }
    let mut promoted = Vec::with_capacity(count * 4);
    for bytes in raw.chunks_exact(2) {
        let value = f16::from_le_bytes([bytes[0], bytes[1]]).to_f32();
        if !value.is_finite() {
            return Err(rejected());
        }
        promoted.extend_from_slice(&value.to_le_bytes());
    }
    tensor.raw_data = Some(promoted);
    tensor.data_type = Some(FLOAT);
    Ok(1)
}

fn rejected() -> RuntimeError {
    RuntimeError::ArtifactMismatch
}

#[cfg(test)]
mod tests;
