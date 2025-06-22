import time
from generator import generate_log
from writer import append_to_stream

batch_interval_sec = 60
records_per_second = 3
batch_records = []

start_time = time.time()
print("🚀 Generating 3 OTEL logs/sec — batching every 60s and sending to Poros Flight server...")

while True:
    batch_records.extend([generate_log() for _ in range(records_per_second)])
    time.sleep(1)

    if time.time() - start_time >= batch_interval_sec:
        print(f"\n📦 Writing {len(batch_records)} records...")
        # write_arrow_file(batch_records)
        append_to_stream(batch_records)
        batch_records = []
        start_time = time.time()