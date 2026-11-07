use super::*;

#[test]
fn promotion_preserves_weight_values_and_rejects_incomplete_or_external_data() {
    let mut tensor = TensorProto {
        data_type: Some(FLOAT16),
        dims: vec![3],
        raw_data: Some(vec![0x00, 0x3c, 0x00, 0xc0, 0x01, 0x00]),
        ..Default::default()
    };
    assert_eq!(promote_tensor(&mut tensor).unwrap(), 1);
    let weights: Vec<_> = tensor
        .raw_data
        .as_ref()
        .unwrap()
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    assert_eq!(weights, [1.0, -2.0, 2_f32.powi(-24)]);
    assert_eq!(tensor.data_type, Some(FLOAT));

    tensor.data_type = Some(FLOAT16);
    assert!(matches!(
        promote_tensor(&mut tensor),
        Err(RuntimeError::ArtifactMismatch)
    ));
    tensor.data_type = Some(FLOAT);
    tensor.data_location = Some(1);
    assert!(matches!(
        promote_tensor(&mut tensor),
        Err(RuntimeError::ArtifactMismatch)
    ));
}
