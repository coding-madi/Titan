import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys
import time

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class SimpleFlightClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)
        logger.info(f"Flight client initialized, connecting to {self.location}")

    def send_data(self, dataset_path: str, table: pa.Table):
        logger.info(f"Attempting to send data for dataset: '{dataset_path}'")
        descriptor = flight.FlightDescriptor.for_path(dataset_path)

        try:
            writer, reader = self.client.do_put(descriptor, table.schema)
            with writer:
                writer.write_table(table)
                logger.info(f"Sent {table.num_rows} rows for '{dataset_path}'.")
            _ = reader.read() # Read any metadata response
            logger.info(f"Data successfully 'do_put' for '{dataset_path}'.")
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
                    # The ticket is the key to retrieving data with do_get
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
        return [f.descriptor for f in flights if f.descriptor.path] # Return descriptors for easy do_get

    def do_get_data(self, dataset_path: str) -> pa.Table | None:
        """
        Retrieves data for a given dataset path using do_get.
        The dataset_path must match the ticket used by the server.
        """
        logger.info(f"\n--- Attempting to retrieve data for '{dataset_path}' using do_get ---")
        
        # The ticket sent to do_get must match what the server provided in FlightInfo.
        # Your Rust server typically uses the dataset path itself as the ticket.
        ticket = flight.Ticket(dataset_path.encode('utf-8')) 

        try:
            # do_get returns a FlightStreamReader
            reader = self.client.do_get(ticket)
            
            # Read all record batches into a single PyArrow Table
            table = reader.read_all()
            
            logger.info(f"Successfully retrieved {table.num_rows} rows from '{dataset_path}'.")
            logger.info(f"Retrieved Table Schema:\n{table.schema.to_string()}")
            logger.info(f"Retrieved Table Data (first 5 rows):\n{table.to_pandas().head()}") # Using pandas for pretty print
            return table
        except flight.FlightError as e:
            logger.error(f"Failed to retrieve data for '{dataset_path}': {e}")
            return None
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_get for '{dataset_path}': {e}")
            return None


    def send_data_with_timing(self, dataset_path: str, table: pa.Table):
        descriptor = flight.FlightDescriptor.for_path(dataset_path)
        try:
            writer, reader = self.client.do_put(descriptor, table.schema)
            start_time = time.time()
            with writer:
                writer.write_table(table)
            _ = reader.read()
            elapsed = time.time() - start_time

            sink = pa.BufferOutputStream()
            with pa.ipc.new_stream(sink, table.schema) as stream_writer:
                stream_writer.write_table(table)
            total_bytes = sink.getvalue().size
            size_mb = total_bytes / (1024 * 1024)
            throughput = size_mb / elapsed if elapsed > 0 else 0

            logger.info(f"Batch '{dataset_path}' upload complete: {size_mb:.2f} MB in {elapsed:.2f}s ({throughput:.2f} MB/s)")
            return True
        except Exception as e:
            logger.error(f"Upload failed for '{dataset_path}': {e}")
            return False

    def generate_batch_table(self, batch_index: int, num_rows: int) -> pa.Table:
        return pa.Table.from_arrays(
            [
                pa.array(range(batch_index * num_rows, (batch_index + 1) * num_rows), type=pa.int64()),
                pa.array(["event"] * num_rows, type=pa.string())
            ],
            names=["event_id", "event_type"]
        )


if __name__ == "__main__":
    SERVER_HOST = "127.0.0.1"
    SERVER_PORT = 50051

    client = SimpleFlightClient(SERVER_HOST, SERVER_PORT)

    # --- Step 1: Perform do_put operations to add data to the server ---
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

    # Give the server a tiny moment to ensure data is processed, though often not strictly necessary.
    time.sleep(0.1) 

    # --- Step 2: Call list_flights to discover available datasets ---
    logger.info("Now calling list_flights to see what's available...")
    # This call now returns the descriptors for easier subsequent do_get calls
    listed_descriptors = client.list_flights()

    # --- Step 3: Perform do_get operations for a specific dataset ---
    # We will try to retrieve the first dataset we put
    if listed_descriptors:
        # Assuming the first listed descriptor is one we just put
        # We need the full path string from the descriptor to use as the ticket
        target_path_segments = [s.decode('utf-8') for s in listed_descriptors[0].path]
        target_dataset_path = "/".join(target_path_segments)
        
        retrieved_table = client.do_get_data(target_dataset_path)
        if retrieved_table:
            logger.info(f"Verification: Successfully retrieved data for '{target_dataset_path}'.")
        else:
            logger.error(f"Verification: Failed to retrieve data for '{target_dataset_path}'.")
    else:
        logger.warning("No datasets listed to attempt do_get.")

    # Try to retrieve the second dataset specifically
    client.do_get_data("/reports/daily_sales")

    logger.info("\nScript finished.")
    # --- Send 10 batches of ~20MB each ---
    rows_per_batch = 1_310_72  # \u2248 20 MB per batch
    for i in range(1000):
        batch_path = f"/benchmark/batch_{i}"
        batch_table = client.generate_batch_table(i, rows_per_batch)
        logger.info(f"Generated batch {i} with {batch_table.num_rows} rows (~20MB)")
        client.send_data_with_timing(batch_path, batch_table)
