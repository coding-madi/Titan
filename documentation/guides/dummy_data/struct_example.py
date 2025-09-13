import pyarrow as pa
import pyarrow.flight as flight
import asyncio
import time
import random
import string
import datetime
from decimal import Decimal, getcontext

getcontext().prec = 18


def random_string(length=50):
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))


def make_schema() -> pa.Schema:
    top_level_fields = []
    nested_fields = []
    field_id_counter = 0

    # --- Add log_group_name and log_name first ---
    field_id_counter += 1
    top_level_fields.append(
        pa.field("log_group_name", pa.string(), metadata={"PARQUET:field_id": str(field_id_counter)})
    )
    field_id_counter += 1
    top_level_fields.append(
        pa.field("log_name", pa.string(), metadata={"PARQUET:field_id": str(field_id_counter)})
    )

    # Primitive fields (20+20+20+10+10+5 = 85 fields)
    for i in range(20):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"int_field_{i}", pa.int64(), metadata={"PARQUET:field_id": str(field_id_counter)}))
    for i in range(20):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"float_field_{i}", pa.float64(), metadata={"PARQUET:field_id": str(field_id_counter)}))
    for i in range(20):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"str_field_{i}", pa.string(), metadata={"PARQUET:field_id": str(field_id_counter)}))
    for i in range(10):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"bool_field_{i}", pa.bool_(), metadata={"PARQUET:field_id": str(field_id_counter)}))
    for i in range(10):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"ts_field_{i}", pa.timestamp("us"), metadata={"PARQUET:field_id": str(field_id_counter)}))
    for i in range(5):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"decimal_field_{i}", pa.decimal128(18, 4), metadata={"PARQUET:field_id": str(field_id_counter)}))

    # Struct fields (5 parent fields)
    for i in range(5):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"struct_field_{i}", pa.null(), metadata={"PARQUET:field_id": str(field_id_counter)}))

    # List fields (5 parent fields)
    for i in range(5):
        field_id_counter += 1
        top_level_fields.append(
            pa.field(f"list_field_{i}", pa.null(), metadata={"PARQUET:field_id": str(field_id_counter)}))

    # Define nested fields
    for i in range(5):
        field_id_counter += 1
        nested_fields.append(
            pa.field(f"nested_int_{i}", pa.int32(), metadata={"PARQUET:field_id": str(field_id_counter)}))
        field_id_counter += 1
        nested_fields.append(
            pa.field(f"nested_str_{i}", pa.string(), metadata={"PARQUET:field_id": str(field_id_counter)}))

    for i in range(5):
        field_id_counter += 1
        nested_fields.append(pa.field("element", pa.int32(), metadata={"PARQUET:field_id": str(field_id_counter)}))

    # Reconstruct schema with correct struct/list types
    final_fields = {}
    nested_idx = 0
    for field in top_level_fields:
        if field.name.startswith("struct_field_"):
            struct_fields = pa.struct([nested_fields[nested_idx], nested_fields[nested_idx + 1]])
            final_fields[field.name] = pa.field(field.name, struct_fields, metadata=field.metadata)
            nested_idx += 2
        elif field.name.startswith("list_field_"):
            list_field = pa.list_(nested_fields[nested_idx])
            final_fields[field.name] = pa.field(field.name, list_field, metadata=field.metadata)
            nested_idx += 1
        else:
            final_fields[field.name] = field

    return pa.schema(list(final_fields.values()))


def generate_complex_batch(num_rows: int, schema: pa.Schema) -> pa.Table:
    arrays = []

    # --- New: log_group_name and log_name ---
    log_groups = [f"log_group_{random.choice(['A','B','C','D','E'])}" for _ in range(num_rows)]
    log_names = [f"log_{i}" for i in range(num_rows)]
    arrays.append(pa.array(log_groups, type=pa.string()))
    arrays.append(pa.array(log_names, type=pa.string()))

    # Primitives
    for _ in range(20):
        arrays.append(pa.array([random.randint(0, 1_000_000) for _ in range(num_rows)], type=pa.int64()))
    for _ in range(20):
        arrays.append(pa.array([random.random() * 1_000_000 for _ in range(num_rows)], type=pa.float64()))
    for _ in range(20):
        arrays.append(pa.array([random_string(50) for _ in range(num_rows)], type=pa.string()))
    for _ in range(10):
        arrays.append(pa.array([random.choice([True, False]) for _ in range(num_rows)], type=pa.bool_()))
    for _ in range(10):
        arrays.append(pa.array([datetime.datetime.now() for _ in range(num_rows)], type=pa.timestamp("us")))
    for _ in range(5):
        arrays.append(pa.array([Decimal(f"{random.uniform(0, 1_000_000):.4f}") for _ in range(num_rows)],
                               type=pa.decimal128(18, 4)))

    # Structs
    for i in range(5):
        struct_array = pa.StructArray.from_arrays(
            [
                pa.array([random.randint(0, 1000) for _ in range(num_rows)], type=pa.int32()),
                pa.array([random_string(10) for _ in range(num_rows)], type=pa.string())
            ],
            fields=list(schema.field_by_name(f"struct_field_{i}").type)
        )
        arrays.append(struct_array)

    # Lists
    for i in range(5):
        list_array = pa.array([[random.randint(0, 1000) for _ in range(5)] for _ in range(num_rows)],
                              type=schema.field_by_name(f"list_field_{i}").type)
        arrays.append(list_array)

    return pa.Table.from_arrays(arrays, schema=schema)


def estimate_rows_for_batch(schema: pa.Schema, target_mb: int = 100, sample_rows: int = 1000) -> int:
    sample_table = generate_complex_batch(sample_rows, schema)
    bytes_per_row = sample_table.nbytes / sample_rows
    target_bytes = target_mb * 1024 * 1024
    return max(1, int(target_bytes / bytes_per_row))


class FlightStreamer:
    def __init__(self, host="127.0.0.1", port=50051):
        self.client = flight.FlightClient(f"grpc://{host}:{port}")

    async def stream_batches(self, dataset_base_path: str, num_batches: int, target_mb: int):
        schema = make_schema()
        rows_per_batch = estimate_rows_for_batch(schema, target_mb)
        total_records = 0
        total_bytes_sent = 0
        total_gen_time = 0
        total_send_time = 0

        descriptor = flight.FlightDescriptor.for_path(dataset_base_path)
        writer, reader = self.client.do_put(descriptor, schema)

        with writer:
            for i in range(num_batches):
                start_gen = time.time()
                table = generate_complex_batch(rows_per_batch, schema)
                gen_elapsed = time.time() - start_gen
                total_gen_time += gen_elapsed

                size_mb = table.nbytes / (1024 * 1024)
                start_send = time.time()
                writer.write_table(table)
                send_elapsed = time.time() - start_send
                total_send_time += send_elapsed

                throughput = size_mb / send_elapsed if send_elapsed > 0 else 0
                print(
                    f"Batch {i + 1}/{num_batches}: {size_mb:.2f} MB | Gen: {gen_elapsed:.2f}s | Send: {send_elapsed:.2f}s | Throughput: {throughput:.2f} MB/s")

                total_bytes_sent += table.nbytes
                total_records += rows_per_batch

        _ = reader.read()

        print("\n✅ Streaming Summary")
        print(f"Total records: {total_records}")
        print(f"Total data: {total_bytes_sent / (1024 * 1024):.2f} MB")
        print(f"Total generation time: {total_gen_time:.2f}s")
        print(f"Total send time: {total_send_time:.2f}s")
        print(f"Average throughput (MB/s): {total_bytes_sent / (1024 * 1024) / total_send_time:.2f}")


async def main():
    streamer = FlightStreamer()
    await streamer.stream_batches("log", num_batches=50, target_mb=100)


if __name__ == "__main__":
    asyncio.run(main())