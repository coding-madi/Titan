import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time
from typing import List, Tuple
import asyncio

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class SimpleFlightClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)
        logger.info(f"Flight client initialized, connecting to {self.location}")
        self.bulk_op_metrics: List[Tuple[int, float]] = []

    def send_data(self, dataset_path: str, table: pa.Table, is_benchmark_op: bool = False):
        descriptor = flight.FlightDescriptor.for_path(dataset_path)
        try:
            writer, reader = self.client.do_put(descriptor, table.schema)
            start_time = time.time()
            with writer:
                writer.write_table(table)
            _ = reader.read()
            elapsed_time = time.time() - start_time

            sink = pa.BufferOutputStream()
            with pa.ipc.new_stream(sink, table.schema) as stream_writer:
                stream_writer.write_table(table)
            bytes_sent_current_op = sink.getvalue().size

            if is_benchmark_op:
                self.bulk_op_metrics.append((bytes_sent_current_op, elapsed_time))

            size_mb = bytes_sent_current_op / (1024 * 1024)
            throughput = size_mb / elapsed_time if elapsed_time > 0 else 0
            return True
        except flight.FlightError as e:
            logger.error(f"Failed to 'do_put' data for '{dataset_path}': {e}")
            return False
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_put for '{dataset_path}': {e}")
            return False

    def list_flights(self, criteria: bytes = b""):
        logger.info(f"\n--- Requesting list of flights with criteria: '{criteria.decode('utf-8', errors='ignore')}' ---")
        found_flights = False
        try:
            flights = self.client.list_flights(criteria)
            flight_count = 0
            for flight_info in flights:
                flight_count += 1
                found_flights = True
                if flight_info.descriptor.path:
                    path_segments = [segment.decode('utf-8', errors='ignore') for segment in flight_info.descriptor.path]
                    path_str = "/".join(path_segments)
                else:
                    path_str = 'N/A'
                logger.info(f"  Found Flight: '{path_str}'")
                logger.info(f"    Schema:\n{flight_info.schema.to_string()}")
                logger.info(f"    Total Records: {flight_info.total_records}")
                logger.info(f"    Total Bytes: {flight_info.total_bytes}")
                logger.info(f"    Endpoints: {len(flight_info.endpoints)}")
                for i, endpoint in enumerate(flight_info.endpoints):
                    ticket_bytes = endpoint.ticket.ticket if endpoint.ticket else b'N/A'
                    ticket_str = ticket_bytes.decode('utf-8', errors='ignore')
                    logger.info(f"      Endpoint {i+1}:")
                    logger.info(f"        Ticket: {ticket_str}")
                    logger.info(f"        Locations: {', '.join(loc.uri for loc in endpoint.locations) if endpoint.locations else 'None'}")
                logger.info("-" * 30)
            if not found_flights:
                logger.warning("No flights listed by the server matching the criteria.")
            else:
                logger.info(f"Successfully listed {flight_count} flights.")
        except flight.FlightError as e:
            logger.error(f"Failed to list flights: {e}")
        except Exception as e:
            logger.error(f"An unexpected error occurred during list_flights: {e}")
        logger.info("--- Flight listing complete ---\n")
        return [f.descriptor for f in flights if f.descriptor.path]

    def do_get_data(self, dataset_path: str) -> pa.Table | None:
        logger.info(f"\n--- Attempting to retrieve data for '{dataset_path}' using do_get ---")
        ticket = flight.Ticket(dataset_path.encode('utf-8'))
        try:
            reader = self.client.do_get(ticket)
            table = reader.read_all()
            logger.info(f"Successfully retrieved {table.num_rows} rows from '{dataset_path}'.")
            logger.info(f"Retrieved Table Schema:\n{table.schema.to_string()}")
            return table
        except flight.FlightError as e:
            logger.error(f"Failed to retrieve data for '{dataset_path}': {e}")
            return None
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_get for '{dataset_path}': {e}")
            return None

    def generate_batch_table(self, batch_index: int, num_rows: int) -> pa.Table:
        element_field = pa.field(
            "element",
            pa.string(),
            nullable=True,
            metadata={"PARQUET:field_id": "4"}
        )
        fields = [
            pa.field("event_id", pa.int64(), nullable=True, metadata={"PARQUET:field_id": "1"}),
            pa.field("event_type", pa.string(), nullable=True, metadata={"PARQUET:field_id": "2"}),
            # tags list with unique field ids
            pa.field(
                "tags",
                pa.list_(
                    pa.field(
                        "element",  # Must match what Iceberg expects
                        pa.string(),
                        metadata={"PARQUET:field_id": "4"}
                    )
                ),
                metadata={"PARQUET:field_id": "3"}  # Unique from everything else
            )
        ]
        schema = pa.schema(fields)
        tags_array = pa.array([["tag1", f"tag{(i % 5) + 1}"] for i in range(batch_index * num_rows, (batch_index + 1) * num_rows)],
                              type=pa.list_(pa.string()))
        return pa.Table.from_arrays(
            [
                pa.array(range(batch_index * num_rows, (batch_index + 1) * num_rows), type=pa.int64()),
                pa.array(["event"] * num_rows, type=pa.string()),
                tags_array
            ],
            schema=schema
        )

    def calculate_bulk_metrics(self) -> Tuple[float, float, int, float, float]:
        total_bytes_sent = sum(item[0] for item in self.bulk_op_metrics)
        total_time_spent = sum(item[1] for item in self.bulk_op_metrics)
        total_successful_ops = len(self.bulk_op_metrics)

        overall_total_bytes_mb = total_bytes_sent / (1024 * 1024)
        if total_time_spent > 0:
            overall_throughput_mbps = overall_total_bytes_mb / total_time_spent
            overall_rate_ops_per_sec = total_successful_ops / total_time_spent
        else:
            logger.warning("Total time spent is 0, defaulting throughput and rate to 0")
            overall_throughput_mbps = 0
            overall_rate_ops_per_sec = 0

        return overall_total_bytes_mb, total_time_spent, total_successful_ops, overall_throughput_mbps, overall_rate_ops_per_sec


async def async_send_batches(client: SimpleFlightClient, batch_descriptor: flight.FlightDescriptor, schema: pa.Schema, rows_per_batch: int, num_batches: int) -> bool:
    all_batches = []
    for i in range(num_batches):
        batch_table = client.generate_batch_table(i, rows_per_batch)
        all_batches.extend(batch_table.to_batches())

    total_bytes = 0
    try:
        sink = pa.BufferOutputStream()
        with pa.ipc.new_stream(sink, schema) as stream_writer:
            for batch in all_batches:
                stream_writer.write_batch(batch)
        total_bytes = sink.getvalue().size
    except Exception as e:
        logger.error(f"Failed to calculate total bytes for batches: {e}")
        return False

    success = False
    start_time = time.time()

    try:
        def blocking_send_operation():
            nonlocal success
            writer, reader = client.client.do_put(batch_descriptor, schema)
            try:
                for batch in all_batches:
                    writer.write_batch(batch)
                writer.close()
                _ = reader.read()
                success = True
            except flight.FlightError as e:
                path_str = "/".join(p.decode("utf-8", errors="ignore") for p in batch_descriptor.path)
                logger.error(f"Flight error during async do_put for '{path_str}': {e}")
                success = False
            except Exception as e:
                path_str = "/".join(p.decode("utf-8", errors="ignore") for p in batch_descriptor.path)
                logger.error(f"Unexpected error during async do_put for '{path_str}': {e}")
                success = False
            finally:
                try:
                    writer.close()
                except Exception as close_e:
                    logger.warning(f"Error closing Flight writer: {close_e}")
        await asyncio.to_thread(blocking_send_operation)
    except Exception as e:
        path_str = "/".join(p.decode("utf-8", errors="ignore") for p in batch_descriptor.path)
        logger.error(f"Error initiating blocking send operation for '{path_str}': {e}")
        success = False

    elapsed_time = time.time() - start_time
    throughput = (total_bytes / (1024 * 1024)) / elapsed_time if elapsed_time > 0 else 0

    path_str = "/".join(p.decode("utf-8", errors="ignore") for p in batch_descriptor.path)
    if success:
        logging.info(f"\n--- Single Stream Metrics (server upload only) ---")
        logging.info(f"Total batches sent: {num_batches}")
        logging.info(f"Total data sent: {total_bytes / (1024 * 1024):.2f} MB")
        logging.info(f"Elapsed time (server upload): {elapsed_time:.4f} seconds")
        logging.info(f"Throughput: {throughput:.2f} MB/s")
    else:
        logging.error(f"\n--- Single Stream Metrics (server upload FAILED) ---")
        logging.error(f"Failed to send all batches for '{path_str}'.")
        logging.error(f"Attempted to send {num_batches} batches, {total_bytes / (1024 * 1024):.2f} MB in {elapsed_time:.4f} seconds.")

    return success


async def main():
    SERVER_HOST = "127.0.0.1"
    SERVER_PORT = 50051
    client = SimpleFlightClient(SERVER_HOST, SERVER_PORT)

    batch_descriptor = flight.FlightDescriptor.for_path("log_list")
    rows_per_batch = 101072
    num_batches = 50

    logging.info(f"\n--- Starting async benchmark for sending {num_batches} batches ---")

    schema = client.generate_batch_table(0, rows_per_batch).schema

    success = await async_send_batches(client, batch_descriptor, schema, rows_per_batch, num_batches)
    path_str = "/".join(p.decode("utf-8", errors="ignore") for p in batch_descriptor.path)
    if not success:
        logger.error(f"Asynchronous batch sending failed for path: {path_str}")
    else:
        logger.info(f"Asynchronous batch sending completed successfully for path: {path_str}")


if __name__ == "__main__":
    asyncio.run(main())
