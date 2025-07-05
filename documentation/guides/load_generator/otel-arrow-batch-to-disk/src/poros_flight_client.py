import pyarrow as pa
import pyarrow.flight as flight
import os
from schema import OTEL_ARROW_SCHEMA

POROS_FLIGHT_ADDR = "grpc+tcp://127.0.0.1:50051"

class PorosFlightClient:
    def __init__(self, address=POROS_FLIGHT_ADDR):
        self.client = flight.FlightClient(address)

    def send_batch(self, batch):
        descriptor = flight.FlightDescriptor.for_path("otel_logs")
        writer, _ = self.client.do_put(descriptor, batch.schema)
        writer.write_batch(batch)
        writer.close()
        print(f"🚀 Sent batch of {batch.num_rows} records to Poros Flight server at {POROS_FLIGHT_ADDR}")
