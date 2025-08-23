import pyarrow.flight as fl

def list_flights(host="localhost", port=50051):
    # Create a Flight client
    client = fl.connect(f"grpc://{host}:{port}")

    print(f"Listing flights from {host}:{port}")
    # List available flights (tables, streams, etc.)
    for flight in client.list_flights():
        print("----")
        print("Descriptor:", flight.descriptor)
        print("Total records (if known):", flight.total_records)
        print("Total bytes (if known):", flight.total_bytes)
        for endpoint in flight.endpoints:
            print("  Ticket:", endpoint.ticket)
            for loc in endpoint.locations:
                print("  Location:", loc)

if __name__ == "__main__":
    list_flights("localhost", 50051)
