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
            throughput = size_mb / elapsed_time if elapsed_time > 0 else 0

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
        overall_throughput_mbps = overall_total_bytes_mb / total_time_spent if total_time_spent > 0 else 0
        overall_rate_ops_per_sec = total_successful_ops / total_time_spent if total_time_spent > 0 else 0

        return overall_total_bytes_mb, total_time_spent, total_successful_ops, overall_throughput_mbps, overall_rate_ops_per_sec

if __name__ == "__main__":
    SERVER_HOST = "127.0.0.1"
    SERVER_PORT = 50051

    client = SimpleFlightClient(SERVER_HOST, SERVER_PORT)

    # --- Initial non-benchmark operations ---
    # These operations will log individually but won't be part of the bulk metrics
    # as `is_benchmark_op` is not set to True.
    dataset_name_1 = "/my_app/user_sessions"
    table_1 = pa.Table.from_arrays(
        [
            pa.array([1, 2, 3], type=pa.int64()),
            pa.array(["login", "logout", "purchase"], type=pa.string()),
            pa.array([time.time()-100, time.time()-50, time.time()], type=pa.float64())
        ],
        names=["user_id", "event", "timestamp"]
    )
    logger.info(f"Created table for '{dataset_name_1}' with {table_1.num_rows} rows.")
    client.send_data(dataset_name_1, table_1)

    dataset_name_2 = "/reports/daily_sales"
    table_2 = pa.Table.from_arrays(
        [
            pa.array(["A", "B"], type=pa.string()),
            pa.array([150.75, 200.00], type=pa.float32()),
        ],
        names=["product_category", "revenue"]
    )
    logger.info(f"Created table for '{dataset_name_2}' with {table_2.num_rows} rows.")
    client.send_data(dataset_name_2, table_2)

    time.sleep(0.1) 

    logger.info("Now calling list_flights to see what's available...")
    listed_descriptors = client.list_flights()

    if listed_descriptors:
        target_path_segments = [s.decode('utf-8') for s in listed_descriptors[0].path]
        target_dataset_path = "/".join(target_path_segments)
        client.do_get_data(target_dataset_path)
    else:
        logger.warning("No datasets listed to attempt do_get.")

    client.do_get_data("/reports/daily_sales")

    # --- Send 1000 batches of ~2MB each (benchmark phase) ---
    rows_per_batch = 131072 # Roughly 2MB per batch
    num_benchmark_batches = 100 # Send 1000 batches for benchmark

    logger.info(f"\n--- Starting benchmark for sending {num_benchmark_batches} batches ---")
    
    # Measure the total wall-clock time for the entire benchmark loop
    overall_benchmark_start_time = time.time() 

    for i in range(num_benchmark_batches):
        batch_path = f"/benchmark/batch_{i}"
        batch_table = client.generate_batch_table(i, rows_per_batch)
        # Pass `is_benchmark_op=True` to collect metrics for bulk calculation
        client.send_data(batch_path, batch_table, is_benchmark_op=True) 
    
    overall_benchmark_end_time = time.time()
    total_wall_clock_time = overall_benchmark_end_time - overall_benchmark_start_time

    logger.info("\nScript finished.")

    # --- Calculate and print overall throughput and rate for the benchmark operations ---
    (total_bytes_mb, total_time_spent_sending, total_successful_sends, 
     overall_throughput_mbps, overall_rate_ops_per_sec) = client.calculate_bulk_metrics()

    logger.info(f"\n--- Overall Performance Metrics (Benchmark do_put operations) ---")
    logger.info(f"Total data sent (benchmark): {total_bytes_mb:.2f} MB")
    logger.info(f"Total time spent actively sending data (benchmark): {total_time_spent_sending:.4f} seconds")
    logger.info(f"Total successful do_put operations (benchmark): {total_successful_sends}")
    logger.info(f"Overall Throughput (MB/s): {overall_throughput_mbps:.2f} MB/s")
    logger.info(f"Overall Rate (DoPut ops/sec): {overall_rate_ops_per_sec:.2f} ops/sec")
    logger.info(f"Total benchmark loop wall-clock time: {total_wall_clock_time:.4f} seconds")