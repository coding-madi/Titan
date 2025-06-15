import pyarrow as pa
import pyarrow.flight as flight
import logging
import sys

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class FlightClient:
    """
    A simple Apache Arrow Flight client for sending dummy data.
    """
    def __init__(self, host: str = "127.0.0.1", port: int = 50051):
        """
        Initializes the FlightClient.

        Args:
            host (str): The hostname or IP address of the Flight server.
            port (int): The port number of the Flight server.
        """
        self.location = f"grpc://{host}:{port}"
        self.client = flight.FlightClient(self.location)
        logger.info(f"Flight client initialized, connecting to {self.location}")

    def send_dummy_data(self, dataset_name: str = "my_dummy_dataset"):
        """
        Generates dummy data and sends it to the Flight server using do_put.

        Args:
            dataset_name (str): A descriptor for the dataset being sent.
        """
        logger.info(f"Preparing to send data for dataset: '{dataset_name}'")

        # 1. Create a dummy PyArrow Table
        try:
            table = pa.Table.from_arrays(
                [
                    pa.array([1, 2, 3, 4, 5], type=pa.int64()),
                    pa.array(["apple", "banana", "cherry", "date", "elderberry"], type=pa.string()),
                    pa.array([True, False, True, False, True], type=pa.bool_()),
                    pa.array([1.1, 2.2, 3.3, 4.4, 5.5], type=pa.float32()),
                ],
                names=["id", "item_name", "is_active", "value"]
            )
            logger.info(f"Created PyArrow Table with {table.num_rows} rows and {table.num_columns} columns.")
            logger.debug(f"Table Schema:\n{table.schema}")
        except Exception as e:
            logger.error(f"Error creating PyArrow Table: {e}")
            return

        # 2. Define a FlightDescriptor
        # This tells the server what kind of data operation is being performed.
        # Here, we use a PathDescriptor, indicating data being "put" to a specific path/name.
        descriptor = flight.FlightDescriptor.for_path(dataset_name)

        # 3. Perform the do_put operation
        try:
            # do_put returns a FlightStreamWriter and a FlightMetadataReader
            # We send the schema first, then write the data batches.
            writer, reader = self.client.do_put(descriptor, table.schema)

            with writer:
                # Write the entire table in one go (or you can write in chunks/batches)
                writer.write_table(table)
                logger.info(f"Sent {table.num_rows} rows to Flight server.")

            # Read any metadata response from the server (optional, but good practice)
            # The server might send back acknowledgements or other info.
            # For a simple put, it might be empty or just an acknowledgement.
            metadata = reader.read()
            logger.info(f"Received metadata from server: {metadata}")

            logger.info("Data successfully sent via Flight do_put.")
        except flight.FlightUnauthenticatedError as e:
            logger.error(f"Authentication error: {e}. Check server credentials.")
        except flight.FlightInternalError as e:
            logger.error(f"Server internal error: {e}")
        except flight.FlightError as e:
            logger.error(f"A general Flight error occurred: {e}")
        except Exception as e:
            logger.error(f"An unexpected error occurred during do_put: {e}")

if __name__ == "__main__":
    # You can change these to match your server's address
    server_host = "127.0.0.1"
    server_port = 50051

    client = FlightClient(server_host, server_port)
    client.send_dummy_data("sensor_readings_2025") # Send some dummy sensor data
    client.send_dummy_data("user_logs_qa")       # Send another type of dummy data
