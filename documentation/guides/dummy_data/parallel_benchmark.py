import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time
from typing import List, Tuple
from concurrent.futures import ThreadPoolExecutor, as_completed

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class CompressedBatch:
    def __init__(self, dataset_path: str, compressed_bytes: bytes, num_rows: int):
        self.dataset_path = dataset_path
        self.compressed_bytes = compressed_bytes
        self.num_rows = num_rows

class SimpleFlightClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)

    def send_precompressed_data(self, batch: CompressedBatch) -> Tuple[int, float]:
        try:
            reader = pa.ipc.open_stream(batch.compressed_bytes)
            table = reader.read_all()
            descriptor = flight.FlightDescriptor.for_path(batch.dataset_path)

            writer, reader = self.client.do_put(descriptor, table.schema)
            start_time = time.time()

            with writer:
                writer.write_table(table)
            _ = reader.read()
            elapsed_time = time.time() - start_time

            compressed_size_bytes = len(batch.compressed_bytes)
            compressed_mb = compressed_size_bytes / (1024 * 1024)
            if elapsed_time > 0:
                throughput = compressed_mb / elapsed_time
            else:
                logger.warning(f"[{batch.dataset_path}] Elapsed time is 0, defaulting throughput to 0")
                throughput = 0

            logger.info(f"[{batch.dataset_path}] Sent {batch.num_rows} rows: Compressed={compressed_mb:.2f} MB \
                        in {elapsed_time:.3f}s ({throughput:.2f} MB/s)")
            return compressed_size_bytes, elapsed_time
        except Exception as e:
            logger.error(f"[{batch.dataset_path}] Failed to send data: {e}")
            return 0, 0.0

    @staticmethod
    def generate_batch_table(batch_index: int, num_rows: int = 10_000) -> pa.Table:
        return pa.Table.from_arrays(
            [
                pa.array(range(batch_index * num_rows, (batch_index + 1) * num_rows), type=pa.int64()),
                pa.array(["event"] * num_rows, type=pa.string())
            ],
            names=["event_id", "event_type"]
        )

    @staticmethod
    def compress_table(table: pa.Table) -> bytes:
        sink = pa.BufferOutputStream()
        with pa.ipc.new_stream(sink, table.schema, options=pa.ipc.IpcWriteOptions(compression='lz4')) as writer:
            writer.write_table(table)
        return sink.getvalue()

def parallel_send_batches(host: str, port: int,
                           rows_per_batch: int = 221072,
                           num_batches: int = 100,
                           max_parallel: int = 8):

    logger.info(f"\n--- Preparing batches ---")
    prep_start = time.time()

    compressed_batches: List[CompressedBatch] = []
    for i in range(num_batches):
        table = SimpleFlightClient.generate_batch_table(i, rows_per_batch)
        compressed_bytes = SimpleFlightClient.compress_table(table)
        compressed_batches.append(CompressedBatch(f"/benchmark/batch_{i}", compressed_bytes, table.num_rows))

    clients = [SimpleFlightClient(host, port) for _ in range(max_parallel)]

    prep_end = time.time()
    prep_time = prep_end - prep_start
    logger.info(f"Prepared {len(compressed_batches)} compressed batches in {prep_time:.3f} seconds. Starting parallel sending with {max_parallel} clients.")

    def task(index: int) -> Tuple[int, float]:
        client = clients[index % max_parallel]
        return client.send_precompressed_data(compressed_batches[index])

    send_start = time.time()

    results: List[Tuple[int, float]] = []
    with ThreadPoolExecutor(max_workers=max_parallel) as executor:
        futures = [executor.submit(task, i) for i in range(num_batches)]
        for future in as_completed(futures):
            results.append(future.result())

    send_end = time.time()
    wall_clock_time = send_end - send_start

    total_bytes = sum(r[0] for r in results)
    total_time = sum(r[1] for r in results)
    successful = sum(1 for r in results if r[1] > 0)

    total_mb = total_bytes / (1024 * 1024)
    if wall_clock_time > 0:
        throughput = total_mb / wall_clock_time
        rate = successful / wall_clock_time
    else:
        logger.warning("Wall-clock time is 0, defaulting throughput and rate to 0")
        throughput = 0
        rate = 0

    logger.info("\n--- Parallel Benchmark Summary ---")
    logger.info(f"Total data sent: {total_mb:.2f} MB")
    logger.info(f"Total sending time (sum of all threads): {total_time:.3f} s")
    logger.info(f"Wall-clock time for sending: {wall_clock_time:.3f} s")
    logger.info(f"Preparation time (batch creation & compression): {prep_time:.3f} s")
    logger.info(f"Successful sends: {successful}")
    logger.info(f"Throughput (excluding prep): {throughput:.2f} MB/s")
    logger.info(f"Send rate (excluding prep, using wall-clock): {rate:.2f} ops/s")

if __name__ == "__main__":
    parallel_send_batches(host="127.0.0.1", port=50051,
                          rows_per_batch=221072,
                          num_batches=100,
                          max_parallel=8)
