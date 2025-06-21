import pyarrow as pa
import pyarrow.flight as flight
import threading
import time
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("FlightServer")

class SimpleFlightServer(flight.FlightServerBase):
    def __init__(self, host="0.0.0.0", port=50051):
        super().__init__(location=f"grpc://{host}:{port}")
        self._tables = {}  # Store tables by dataset path
        self.location = f"grpc://{host}:{port}"  # Ensure self.location is set for list_flights
        logger.info(f"Flight server will listen on {host}:{port}")

    def do_put(self, context, descriptor, reader, writer):
        table = reader.read_all()
        # Fix: decode bytes to str for path segments
        path = "/".join(s.decode() if isinstance(s, bytes) else s for s in descriptor.path)
        self._tables[path] = table
        logger.info(f"Received table for path: {path} with {table.num_rows} rows.")

    def list_flights(self, context, criteria):
        for path, table in self._tables.items():
            # Ensure path is str for FlightDescriptor.for_path (decode if bytes)
            if isinstance(path, bytes):
                path_str = path.decode()
            else:
                path_str = path
            yield flight.FlightInfo(
                table.schema,
                flight.FlightDescriptor.for_path(path_str),
                [flight.FlightEndpoint(flight.Ticket(path_str.encode()), [self.location])],
                table.num_rows,
                table.nbytes if hasattr(table, 'nbytes') else 0
            )

    def do_get(self, context, ticket):
        path = ticket.ticket.decode()
        table = self._tables.get(path)
        if table is None:
            raise flight.FlightUnavailableError(f"No data for path: {path}")
        logger.info(f"Sending table for path: {path} with {table.num_rows} rows.")
        return flight.RecordBatchStream(table)

def serve():
    server = SimpleFlightServer()
    server_thread = threading.Thread(target=server.serve, daemon=True)
    server_thread.start()
    logger.info("Flight server started. Press Ctrl+C to stop.")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Shutting down Flight server...")
        server.shutdown()

if __name__ == "__main__":
    serve()
