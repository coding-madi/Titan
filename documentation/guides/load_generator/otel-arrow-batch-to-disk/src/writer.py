import pyarrow as pa
import pyarrow.ipc as ipc
import os
from datetime import datetime
from schema import OTEL_ARROW_SCHEMA
from poros_flight_client import PorosFlightClient

ARROW_DIR = "files"
ARROW_FILE_PREFIX = "otel_logs_"
STREAM_FILE_PATH = os.path.join(ARROW_DIR, "otel_logs.stream")

# def write_arrow_file(batch_records):
#     if not os.path.exists(ARROW_DIR):
#         os.makedirs(ARROW_DIR)
#     filename = datetime.utcnow().strftime(f"{ARROW_FILE_PREFIX}%Y%m%d_%H%M.arrow")
#     filepath = os.path.join(ARROW_DIR, filename)
#     batch = _records_to_batch(batch_records)
#
#     with ipc.new_file(filepath, OTEL_ARROW_SCHEMA) as writer:
#         writer.write(batch)
#     print(f"✅ Wrote batch to {filepath}")

def append_to_stream(batch_records):
    batch = _records_to_batch(batch_records)
    client = PorosFlightClient()
    client.send_batch(batch)
    print(f"📡 Appended {len(batch_records)} records to Poros Flight server")

def _records_to_batch(records):
    return pa.record_batch([
        pa.array([r["time_unix_nano"] for r in records], type=pa.int64()),
        pa.array([r["severity_number"] for r in records], type=pa.int32()),
        pa.array([r["severity_text"] for r in records], type=pa.string()),
        pa.array([r["body"] for r in records], type=pa.string()),
        pa.array([r["trace_id"] for r in records], type=pa.binary(16)),
        pa.array([r["span_id"] for r in records], type=pa.binary(8)),
        pa.array([r["attributes"] for r in records], type=pa.map_(pa.string(), pa.string()))
    ], schema=OTEL_ARROW_SCHEMA)