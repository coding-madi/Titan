import struct
import os
import pyarrow.ipc as ipc
import pyarrow as pa
import hashlib # For a simple placeholder checksum

WAL_FILE_PATH = "/tmp/wal_entry.log"
# **IMPORTANT**: This must precisely match the *packed* size of your Rust WalBlockHeader.
# 8 (magic) + 8 (metadata_offset) + 2 (metadata_length) + 4 (checksum) +
# 8 (reserve_offset) + 8 (reserve_length) + 8 (data_offset) +
# 8 (data_length) + 8 (total_block_size) + 1 (padding) = 63 bytes.
WAL_BLOCK_HEADER_SIZE = 63
WAL_MAGIC = b"WALBLOCK"

# --- Placeholder for Rust's checksum computation ---
# **CRITICAL**: This Python function MUST EXACTLY replicate the checksum algorithm
# used in your Rust `cksum::compute_checksum`.
# For example, if Rust uses `crc32fast`, you'd use `zlib.crc32` in Python.
# If it's a custom hash or another library, you must port that logic.
# The current SHA256 is for demonstration and will likely NOT match.
def compute_checksum_py(metadata_bytes: bytes, data_bytes: bytes) -> int:
    """
    Placeholder for the Rust `cksum::compute_checksum` function.
    Assumes SHA256, taking the first 4 bytes for a u32 checksum.
    """
    hasher = hashlib.sha256()
    hasher.update(metadata_bytes)
    hasher.update(data_bytes)
    # Rust's u32 checksum means we need a 4-byte integer from the hash.
    return int.from_bytes(hasher.digest()[:4], 'big')


# --- WalBlockHeader struct format (must match Rust's #[repr(C, packed)] layout) ---
# <   : Little-endian
# 8s  : magic: [u8; 8]
# Q   : metadata_offset: u64
# H   : metadata_length: u16
# I   : checksum: u32
# Q   : reserve_offset: u64
# Q   : reserve_length: u64
# Q   : data_offset: u64
# Q   : data_length: u64
# Q   : total_block_size: u64
# B   : padding: u8
WAL_HEADER_STRUCT_FORMAT = "<8s Q H I Q Q Q Q Q B"

def validate_wal_file(file_path: str):
    print(f"Starting validation of WAL file: {file_path}")
    if not os.path.exists(file_path):
        print(f"Error: File not found at {file_path}")
        return

    total_blocks_validated = 0
    total_bytes_processed = 0

    with open(file_path, "rb") as f:
        while True:
            current_pos = f.tell()
            header_bytes = f.read(WAL_BLOCK_HEADER_SIZE)

            if not header_bytes: # End of file
                break
            if len(header_bytes) < WAL_BLOCK_HEADER_SIZE:
                print(f"Warning: Partial header at offset {current_pos}. File might be truncated or corrupted.")
                break

            try:
                # Unpack the header fields according to the defined format
                (
                    magic,
                    metadata_offset,
                    metadata_length,
                    checksum,
                    reserve_offset,
                    reserve_length,
                    data_offset,
                    data_length,
                    total_block_size,
                    padding_byte # This is the single padding byte from Rust struct
                ) = struct.unpack(WAL_HEADER_STRUCT_FORMAT, header_bytes)

                # --- VALIDATE HEADER FIELDS ---
                if magic != WAL_MAGIC:
                    print(f"Error at offset {current_pos}: Invalid magic number. Expected {WAL_MAGIC}, Got {magic}")
                    break

                # Basic sanity checks on sizes and offsets
                if total_block_size < WAL_BLOCK_HEADER_SIZE + metadata_length + data_length:
                     print(f"Error at offset {current_pos}: Declared total_block_size ({total_block_size}) is smaller than sum of components ({WAL_BLOCK_HEADER_SIZE + metadata_length + data_length}).")
                     break

                # The `metadata_offset` and `data_offset` in your Rust header are offsets *relative to the start of the block*.
                # This makes them absolute offsets within the block.
                # `metadata_offset` should be WAL_BLOCK_HEADER_SIZE (63) if metadata immediately follows header.
                # `data_offset` should be WAL_BLOCK_HEADER_SIZE + metadata_length if data immediately follows metadata.

                expected_metadata_start_offset = WAL_BLOCK_HEADER_SIZE
                if metadata_offset != expected_metadata_start_offset:
                    print(f"Warning at offset {current_pos}: Metadata offset in header ({metadata_offset}) does not match expected ({expected_metadata_start_offset}).")

                expected_data_start_offset = WAL_BLOCK_HEADER_SIZE + metadata_length
                if data_offset != expected_data_start_offset:
                    print(f"Warning at offset {current_pos}: Data offset in header ({data_offset}) does not match expected ({expected_data_start_offset}).")


                # --- Read Metadata and Data Payloads ---
                # Seek to start of metadata relative to block start (current_pos)
                f.seek(current_pos + metadata_offset)
                metadata_bytes = f.read(metadata_length)
                if len(metadata_bytes) < metadata_length:
                    print(f"Error at offset {current_pos}: Partial metadata. Expected {metadata_length} bytes, Got {len(metadata_bytes)}")
                    break

                # Seek to start of Arrow data relative to block start (current_pos)
                f.seek(current_pos + data_offset)
                arrow_data_bytes = f.read(data_length)
                if len(arrow_data_bytes) < data_length:
                    print(f"Error at offset {current_pos}: Partial Arrow data. Expected {data_length} bytes, Got {len(arrow_data_bytes)}")
                    break

                # --- CHECKSUM VALIDATION ---
                computed_checksum = compute_checksum_py(metadata_bytes, arrow_data_bytes)
                # if computed_checksum != checksum:
                #     print(f"Error at offset {current_pos}: Checksum mismatch. Expected {checksum}, Computed {computed_checksum}")
                #     # You might want to continue or break depending on error handling strategy
                #     # For now, we break on checksum mismatch as it indicates corruption
                #     break

                # --- ARROW DATA VALIDATION ---
                try:
                    # Create a PyArrow Buffer from the bytes
                    arrow_buffer = pa.py_buffer(arrow_data_bytes)
                    # Open an IPC stream reader
                    reader = ipc.open_stream(arrow_buffer)
                    # Read all record batches. This will throw an error if the format is invalid.
                    record_batches = reader.read_all()
                    print(f"  - Successfully parsed {len(record_batches)} Arrow RecordBatch(es).")
                    for i, batch in enumerate(record_batches):
                        # print(f"    - Batch {i}: {batch.num_rows} rows, {batch.num_columns} columns")
                        print("Looping")
                        # You can inspect the data here: print(batch.to_pydict())

                except Exception as e:
                    print(f"Error at offset {current_pos}: Failed to parse Arrow IPC data: {e}")
                    # You might want to continue or break
                    break

                total_blocks_validated += 1
                total_bytes_processed += total_block_size

                # Move to the start of the next block
                f.seek(current_pos + total_block_size)
                print(f"Validated block at offset {current_pos}. Moving to next block at {f.tell()}")

            except struct.error as e:
                print(f"Error at offset {current_pos}: Failed to unpack header. Malformed data? {e}")
                print(f"Header bytes at offset {current_pos}: {header_bytes.hex()}")
                break
            except Exception as e:
                print(f"An unexpected error occurred at offset {current_pos}: {e}")
                print(f"Header bytes at offset {current_pos}: {header_bytes.hex()}")
                break

    print(f"\n--- Validation Summary ---")
    print(f"Total blocks validated: {total_blocks_validated}")
    print(f"Total bytes processed: {total_bytes_processed} bytes")


if __name__ == "__main__":
    # Ensure WAL_FILE_PATH points to your actual WAL file for testing
    validate_wal_file(WAL_FILE_PATH)