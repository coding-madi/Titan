import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time
import random
import string
import asyncio
from typing import List, Tuple

# Setup logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)


def random_string(length=50):
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))


class SimpleFlightClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)
        logger.info(f"Flight client initialized, connecting to {self.location}")

    def send_data(self, dataset_path: str, table: pa.Table) -> bool:
        descriptor = flight.FlightDescriptor.for_path(dataset_path)
        try:
            writer, reader = self.client.do_put(descriptor, table.schema)
            start_time = time.time()
            with writer:
                writer.write_table(table)
            _ = reader.read()
            elapsed_time = time.time() - start_time

            size_mb = table.nbytes / (1024 * 1024)
            throughput = size_mb / elapsed_time if elapsed_time > 0 else 0

            logger.info(f"Sent {size_mb:.2f} MB in {elapsed_time:.2f} sec "
                        f"({throughput:.2f} MB/s) for '{dataset_path}'")
            return True
        except Exception as e:
            logger.error(f"Failed to send data for '{dataset_path}': {e}")
            return False

    @staticmethod
    def generate_batch_table(batch_index: int, num_rows: int) -> pa.Table:
        # Field definitions with field_id metadata
        event_id_field = pa.field("event_id", pa.int64(), nullable=True,
                                metadata={"PARQUET:field_id": "1"})
        event_type_field = pa.field("event_type", pa.string(), nullable=True,
                                    metadata={"PARQUET:field_id": "2"})

        # Struct field for metadata: struct<tag1: string, tag2: string>
        metadata_field = pa.field("metadata", pa.struct([
            pa.field("tag1", pa.string(), nullable=True,
                    metadata={"PARQUET:field_id": "4"}),
            pa.field("tag2", pa.string(), nullable=True,
                    metadata={"PARQUET:field_id": "5"}),
        ]), nullable=True, metadata={"PARQUET:field_id": "3"})

        schema = pa.schema([event_id_field, event_type_field, metadata_field])

        # Dummy data
        event_ids = pa.array(range(batch_index * num_rows, (batch_index + 1) * num_rows), type=pa.int64())
        event_types = pa.array(["event"] * num_rows, type=pa.string())
        tag1_array = pa.array([f"tag1-{i}" for i in range(batch_index * num_rows, (batch_index + 1) * num_rows)])
        tag2_array = pa.array([f"tag2-{i % 5}" for i in range(batch_index * num_rows, (batch_index + 1) * num_rows)])
        metadata_array = pa.StructArray.from_arrays([tag1_array, tag2_array], fields=metadata_field.type)

        return pa.Table.from_arrays([event_ids, event_types, metadata_array], schema=schema)



async def send_streaming_batches(client: SimpleFlightClient,
                                 dataset_base_path: str,
                                 rows_per_batch: int,
                                 num_batches: int):
    logger.info(f"Starting to send {num_batches} batches (~{rows_per_batch} rows each)")

    total_size_mb = 0.0
    for i in range(num_batches):
        table = client.generate_batch_table(i, rows_per_batch)
        size_mb = table.nbytes / (1024 * 1024)

        if size_mb > 3:
            logger.warning(f"Batch {i} is too large ({size_mb:.2f} MB). Reduce row count!")
            continue

#         path = f"{dataset_base_path}/batch_{i}"
        path = "log"
        success = client.send_data(path, table)
        if not success:
            logger.error(f"❌ Failed to send batch {i}")
            return False
        total_size_mb += size_mb

    logger.info(f"✅ Finished sending all {num_batches} batches.")
    logger.info(f"📦 Total Data Sent: {total_size_mb:.2f} MB")
    return True


async def main():
    client = SimpleFlightClient("127.0.0.1", 50051)

    dataset_base_path = "/benchmark/streamed_batches"
    rows_per_batch = 60000  # Tune this if batch > 3MB
    num_batches = 4000

    await send_streaming_batches(client, dataset_base_path, rows_per_batch, num_batches)


if __name__ == "__main__":
    asyncio.run(main())
