# schema.py
import pyarrow as pa

OTEL_ARROW_SCHEMA = pa.schema([
    ("time_unix_nano", pa.int64()),
    ("severity_number", pa.int32()),
    ("severity_text", pa.string()),
    ("body", pa.string()),
    ("trace_id", pa.binary(16)),
    ("span_id", pa.binary(8)),
    ("attributes", pa.map_(pa.string(), pa.string()))
])