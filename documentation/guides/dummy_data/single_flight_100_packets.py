import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time
from typing import List, Tuple

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class SimpleFlightClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)
        logger.info(f"Flight client initialized, connecting to {self.location}")

        # Metrics accumulators for bulk operations
        self.bulk_op_metrics: List[Tuple[int, float]] = [] # List of (bytes_sent, elapsed_time) for each do_put

    def send_data(self, dataset_path: str, table: pa.Table, is_benchmark_op: bool = False):
        """
        Sends data to the Flight server.
        If is_benchmark_op is True, metrics are collected for bulk calculation.
        """
        descriptor = flight.FlightDescriptor.for_path(dataset_path)

        try:
            writer, reader = self.client.do_put(descriptor, table.schema)
            start_time = time.time()
            with writer:
                writer.write_table(table)
            _ = reader.read() # Read any metadata response
            elapsed_time = time.time() - start_time

            sink = pa.BufferOutputStream()
            with pa.ipc.new_stream(sink, table.schema) as stream_writer:
                stream_writer.write_table(table)
            bytes_sent_current_op = sink.getvalue().size

            if is_benchmark_op:
                self.bulk_op_metrics.append((bytes_sent_current_op, elapsed_time))

            size_mb = bytes_sent_current_op / (1024 * 1024)
            if elapsed_time > 0:
                throughput = size_mb / elapsed_time
            else:
                logger.warning(f"Elapsed time is 0 for '{dataset_path}', defaulting throughput to 0")
                throughput = 0

            # logger.info(f"Sent {table.num_rows} rows for '{dataset_path}'. Data: {size_mb:.2f} MB in {elapsed_time:.4f}s ({throughput:.2f} MB/s)")
            return True
        except flight.FlightError as e:
            logger.error(f"Failed to 'do_put' data for '{dataset_path}': {e}")
            return False
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_put for '{dataset_path}': {e}")
            return False

    def list_flights(self, criteria: bytes = b""):
        # This method remains unchanged as it's for discovery, not bulk data transfer.
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
        # This method remains unchanged as it's for retrieval, not bulk data transfer metrics.
        logger.info(f"\n--- Attempting to retrieve data for '{dataset_path}' using do_get ---")
        
        ticket = flight.Ticket(dataset_path.encode('utf-8')) 

        try:
            reader = self.client.do_get(ticket)
            table = reader.read_all()
            
            logger.info(f"Successfully retrieved {table.num_rows} rows from '{dataset_path}'.")
            logger.info(f"Retrieved Table Schema:\n{table.schema.to_string()}")
            # logger.info(f"Retrieved Table Data (first 5 rows):\n{table.to_pandas().head()}")
            return table
        except flight.FlightError as e:
            logger.error(f"Failed to retrieve data for '{dataset_path}': {e}")
            return None
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_get for '{dataset_path}': {e}")
            return None

    def generate_batch_table(self, batch_index: int, num_rows: int) -> pa.Table:
        return pa.Table.from_arrays(
            [
                pa.array(range(batch_index * num_rows, (batch_index + 1) * num_rows), type=pa.int64()),
                pa.array(["event"] * num_rows, type=pa.string())
            ],
            names=["event_id", "event_type"]
        )

    def calculate_bulk_metrics(self) -> Tuple[float, float, int, float]:
        """
        Calculates and returns total bytes sent (MB), total time (s),
        total successful operations, and overall throughput (MB/s).
        """
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

import asyncio
import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time

async def async_send_batches(client: SimpleFlightClient, batch_descriptor, schema, rows_per_batch, num_batches):
    # Pre-generate all batches before timing
    all_batches = []
    for i in range(num_batches):
        batch_table = client.generate_batch_table(i, rows_per_batch)
        # Convert to record batches (Flight expects record batches)
        all_batches.extend(batch_table.to_batches())

    def blocking_send():
        writer, reader = client.client.do_put(batch_descriptor, schema)
        with writer:
            for batch in all_batches:
                writer.write_batch(batch)
        _ = reader.read()

    start_time = time.time()
    await asyncio.to_thread(blocking_send)
    elapsed_time = time.time() - start_time

    # Calculate total bytes sent for all batches
    sink = pa.BufferOutputStream()
    with pa.ipc.new_stream(sink, schema) as stream_writer:
        for i in range(num_batches):
            stream_writer.write_table(client.generate_batch_table(i, rows_per_batch))
    total_bytes = sink.getvalue().size

    throughput = (total_bytes / (1024 * 1024)) / elapsed_time if elapsed_time > 0 else 0

    logging.info(f"\n--- Single Stream Metrics (server upload only) ---")
    logging.info(f"Total batches sent: {num_batches}")
    logging.info(f"Total data sent: {total_bytes / (1024 * 1024):.2f} MB")
    logging.info(f"Elapsed time (server upload): {elapsed_time:.4f} seconds")
    logging.info(f"Throughput: {throughput:.2f} MB/s")


async def main():
    SERVER_HOST = "127.0.0.1"
    SERVER_PORT = 50051
    client = SimpleFlightClient(SERVER_HOST, SERVER_PORT)

    batch_descriptor = flight.FlightDescriptor.for_path("/benchmark/single_stream_batches")
    rows_per_batch = 131072
    num_batches = 100

    logging.info(f"\n--- Starting async benchmark for sending 100 batches ---")

    schema = client.generate_batch_table(0, rows_per_batch).schema

    await async_send_batches(client, batch_descriptor, schema, rows_per_batch, num_batches)

if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                        format='%(asctime)s - %(levelname)s - %(message)s')
    asyncio.run(main())

