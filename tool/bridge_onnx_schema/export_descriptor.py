"""Export the admitted ONNX schema for the Rust precision adapter."""

from hashlib import sha256
from pathlib import Path

import onnx
from google.protobuf.descriptor_pb2 import FileDescriptorSet
from onnx import onnx_ml_pb2

if onnx.__version__ != "1.20.1":
    raise RuntimeError("Schema generation requires ONNX 1.20.1")
descriptor = FileDescriptorSet()
descriptor.file.add().ParseFromString(onnx_ml_pb2.DESCRIPTOR.serialized_pb)
data = descriptor.SerializeToString()
if sha256(data).hexdigest() != "5423257e29ec20d0243d8cc791be0953768a116893802dbf97db7d6b2758988d":
    raise RuntimeError("ONNX descriptor differs from the admitted schema")
output = Path("build/onnx-schema/descriptor.bin")
output.parent.mkdir(parents=True, exist_ok=True)
output.write_bytes(data)
