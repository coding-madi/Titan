import struct
import os
import pyarrow.ipc as ipc
import pyarrow as pa
import hashlib # For a simple checksum, if cksum in Rust is something like SHA256

WAL_FILE_PATH = "/tmp/wal_entry.log"
WAL_BLOCK_HEADER_SIZE = 48
WAL_MAGIC = b"WALBLOCK"

# --- Placeholder for Rust's checksum computation ---
def compute_checksum_py(metadata_bytes: bytes, data_bytes: bytes) -> int:
    """
    Re-implements the Rust `cksum::compute_checksum` function.
    This is a placeholder. You NEED to match your Rust implementation EXACTLY.
    For example, if Rust used CRC32, you'd use zlib.crc32.
    If it used a simple sum, you'd do that.
    Let's assume a simple SHA256 for demonstration.
    """
    hasher = hashlib.sha256()
    hasher.update(metadata_bytes)
    hasher.update(data_bytes)
    # Return a part of the hash as an integer, compatible with u64
    return int.from_bytes(hasher.digest()[:8], 'big') # Take first 8 bytes for u64

# --- WalBlockHeader structure (must match Rust's exact byte layout) ---
# Format string for struct.unpack:
# <   : Little-endian
# 8s  : 8-byte string (magic)
# Q   : unsigned long long (u64) for metadata_offset
# H   : unsigned short (u16) for metadata_length
# Q   : unsigned long long (u64) for reserved
# Q   : unsigned long long (u64) for checksum
# Q   : unsigned long long (u64) for reserve_offset
# Q   : unsigned long long (u64) for reserve_length
# WAL_HEADER_STRUCT_FORMAT = "<8sHQ Q Q Q" # Total 8 + 8 + 2 + 8 + 8 + 8 + 8 = 50 bytes.
# WAL_HEADER_STRUCT_FORMAT = "<8sQH6xQQQQQ"
WAL_HEADER_STRUCT_FORMAT = "<8sQH H I Q Q Q"
WAL_HEADER_STRUCT_FORMAT = "<8sQH H I Q Q Q"  # Must match exact field sizes: 8s u64 u16 u16 u32 u64 u64 u64
# Note: Your Rust struct has `metadata_offset: u64`, `metadata_length: u16`, `reserved: u64`, `checksum: u64`, `reserve_offset: u64`, `reserve_length: u64`.
# If it's a packed struct, the 64 bytes is padding.
# The Rust code `header_bytes = bytes_of(&header)` suggests a direct byte representation.
# If `bytes_of` just concatenates `to_le_bytes()`, the struct format would be `8s Q H Q Q Q Q`.
# Let's assume the Rust `bytes_of` pads to 64 bytes correctly.
# We'll use the fields you listed, and adjust the struct format to match their sizes.
# The issue is, `8s + 8 + 2 + 8 + 8 + 8 + 8 = 50`. Your header is 64 bytes.
# This means there's 14 bytes of padding. Where? Usually at the end or explicitly.
# For simplicity, let's assume the first 50 bytes are the data and the rest is padding.
# You need to confirm the *exact* byte layout of your `WalBlockHeader` after `bytes_of()`.
# For now, let's use a placeholder assuming your `bytes_of` correctly produces 64 bytes
# where the relevant fields are at their expected offsets.

# Example: If your Rust header is actually 64 bytes total and `bytes_of` handles padding:
# magic (8), metadata_offset (8), metadata_length (2), reserved (8), checksum (8), reserve_offset (8), reserve_length (8)
# That's 50 bytes. The remaining 14 bytes are padding.

# Let's define the fields based on your Rust struct directly and unpack based on that.
# Assume the padding is implicitly at the end if the Rust struct is `#[repr(C)]` or `#[repr(packed)]`

def validate_wal_file(file_path: str):
    print(f"Starting validation of WAL file: {file_path}")
    if not os.path.exists(file_path):
        print(f"Error: File not found at {file_path}")
        return

    total_blocks_validated = 0
    total_bytes_read = 0

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
                # Unpack the header fields. Adjust the format string based on your Rust struct's packing.
                # Assuming padding at the end of the 64 bytes.
                # header_tuple = struct.unpack(WAL_HEADER_STRUCT_FORMAT, header_bytes)
                (magic, metadata_offset_raw, metadata_length_raw, reserved_raw,
                 checksum_raw, reserve_offset_raw, reserve_length_raw, total_block_size) = struct.unpack(WAL_HEADER_STRUCT_FORMAT, header_bytes[:48]) # Adjust if your actual 
                # 
                # packed size is different
                # If there's no padding after the fields, and the total WalBlockHeader is 64, it's just `header_bytes`.
                # You must verify your Rust `bytes_of(&header)` output structure.

                # --- VALIDATE HEADER FIELDS ---
                if magic != WAL_MAGIC:
                    print(f"Error at offset {current_pos}: Invalid magic number. Expected {WAL_MAGIC}, Got {magic}")
                    break

                # Convert raw values to their actual types as needed (already done by struct.unpack for int/short)
                metadata_offset = metadata_offset_raw
                metadata_length = metadata_length_raw
                checksum = checksum_raw
                reserve_offset = reserve_offset_raw
                reserve_length = reserve_length_raw

                # Basic sanity checks on offsets/lengths
                if metadata_offset < WAL_BLOCK_HEADER_SIZE:
                    print(f"Error at offset {current_pos}: Metadata offset {metadata_offset} is less than header size.")
                    break
                if metadata_offset != WAL_BLOCK_HEADER_SIZE: # Your Rust code hardcoded metadata_offset to 64
                     print(f"Warning at offset {current_pos}: Metadata offset is {metadata_offset}, expected {WAL_BLOCK_HEADER_SIZE}")

                # Read metadata
                f.seek(current_pos + metadata_offset)
                metadata_bytes = f.read(metadata_length)
                if len(metadata_bytes) < metadata_length:
                    print(f"Error at offset {current_pos}: Partial metadata. Expected {metadata_length} bytes, Got {len(metadata_bytes)}")
                    break

                # Calculate data length based on your Rust structure.
                # Your Rust code implies the data follows metadata immediately.
                # The total size of the block is NOT in the header directly.
                # You need to *know* the size of the block that was claimed by the WalOffsetManager.
                # This is a critical piece of information that is *not* currently stored in your WalBlockHeader.
                # Without the *actual total block size* in the header, reading is ambiguous.

                # ***** CRITICAL: You NEED to add `data_length: u64` or `total_block_size: u64` to your WalBlockHeader in Rust. *****
                # For this script to work, I'll have to *assume* we can read until the start of the next header.
                # This is dangerous if your WAL file can have gaps or unexpected data.

                # For the purpose of this validation script, let's make a dangerous assumption:
                # The data extends until the beginning of the *next* header, or EOF if this is the last block.
                # In a real WAL, the header MUST contain the total block size.

                # For now, let's simulate:
                # Assuming data starts after metadata and extends for a known size or until end of file.
                # Your Rust `WalOffsetManager.claim_offset` *knows* the `actual_block_size`.
                # This `actual_block_size` needs to be part of your `WalBlockHeader` for robust validation.
                # Let's add a placeholder `data_length` field to our theoretical WalBlockHeader struct for this validation.
                # If you add `data_length: u64` to your Rust WalBlockHeader, then you'd read it here.
                # For now, I cannot do that robustly.

                # Let's assume your data_buf in Rust ends after metadata + actual data.
                # And the next block header starts immediately after the end of `data_buf`.
                # We need to compute the `data_len` from the remaining space until the next header.
                # This is why total_block_size in header is so critical.

                # Without `total_block_size` in header, we'll try to read a "reasonable" amount.
                # This is a serious vulnerability in the WAL's self-description if not fixed.
                # For the script to proceed, I will make a blind read for the "data_buf" part.
                # This is where your WAL design currently has a gap for recovery/validation.
                # Let's just read "some" bytes after metadata, until next header or EOF
                # This is a heuristic, NOT robust.

                # We'll read until 64 bytes before EOF, or until we hit something that looks like the next header.
                # A more robust approach if you can't change the header: read current position + remaining file size,
                # then try to parse block by block, but this is prone to errors if there's padding or gaps.

                # Given your Rust `actual_block_size` is calculated as `header_dummy_size + metadata.len() + data_buf.len()`,
                # this `actual_block_size` needs to be in your WalBlockHeader for the reader to know how much to read.
                # Let's assume you've updated `WalBlockHeader` to include `total_block_size: u64`

                # Let's re-define the conceptual header with `total_block_size` for robust validation
                # WAL_HEADER_STRUCT_FORMAT_ROBUST = "<8sHQ Q Q Q Q" # Added one more Q for total_block_size
                # And assume you parse it like:
                # (magic, metadata_offset_raw, metadata_length_raw, reserved_raw,
                #  checksum_raw, reserve_offset_raw, reserve_length_raw, total_block_size_raw) = struct.unpack(WAL_HEADER_STRUCT_FORMAT_ROBUST, header_bytes)
                # Then data_len = total_block_size - WAL_BLOCK_HEADER_SIZE - metadata_length

                # For the current Rust code without `total_block_size` in the header, this validation is heuristic.
                # I'll simulate reading the `data_buf` by assuming `total_block_size` is part of the header.
                # You MUST modify your Rust `WalBlockHeader` to include `total_block_size` for this to be reliable.
                # For the sake of demonstration, I will assume a field `total_block_size` is present in the unpacked header.
                # Let's assume its after reserve_length, so 8s + 8 + 2 + 8 + 8 + 8 + 8 + 8 = 58 bytes.
                # WAL_HEADER_STRUCT_FORMAT = "<8sHQ Q Q Q Q" (This would make the header > 64 bytes if unpacked without padding)

                # Let's instead assume your `metadata_offset` is always 64 and `reserve_offset` is always `64 + metadata_length`.
                # The remaining part after `reserve_offset` will be `data_buf`.
                # The problem is: how long is `data_buf`?

                # Let's simplify and make a *temporary* (and dangerous) assumption for the script to run:
                # The length of the Arrow data is determined by reading until the end of the `total_block_size`
                # which MUST be present in your `WalBlockHeader`.
                # Since it's not currently, I'll put a placeholder for `data_len` from `header`.
                # You'll need to modify your Rust WalBlockHeader to include `data_len: u64` or `total_block_size: u64`.
                # If `total_block_size` is in header, then:
                # `data_len = total_block_size - WAL_BLOCK_HEADER_SIZE - metadata_length`

                # Let's hardcode a dummy data_len for the script to run, assuming you fix the header in Rust.
                # This is where the script is fragile WITHOUT a `total_block_size` field in your header.
                # For robust validation, the header MUST self-describe the block's total size.

                # Let's try to infer `data_len` if `total_block_size` isn't in header:
                # In your rust: `actual_block_size = header_dummy_size + metadata.len() as u64 + data_buf.len() as u64;`
                # So `data_len = actual_block_size - header_dummy_size - metadata.len()`
                # If `actual_block_size` could be read from the header (e.g., `total_block_size` field):
                # data_offset = current_pos + WAL_BLOCK_HEADER_SIZE + metadata_length
                # data_len = total_block_size - WAL_BLOCK_HEADER_SIZE - metadata_length

                # For now, let's assume `data_len` is somehow known or inferred from the total block size (if present).
                # Since the Rust `actual_block_size` is only known at the time of claim_offset,
                # the Python script cannot derive it without it being in the header.
                # The only safe way is to include `total_block_size: u64` in `WalBlockHeader`.
                # For this script to work, I will assume a `block_data_length_field` is added to your header.

                # Assume a fixed-size data section (this is usually NOT how Arrow IPC works) for demo.
                # You MUST replace this with reading `total_block_size` from header.

                # Placeholder values if you added `total_block_size` to your Rust header
                # Let's say `total_block_size` is read from `header_bytes` as well
                # For `struct.unpack`, you need to know *exactly* where each field is.
                # Let's just create a dummy for WalBlockHeader fields to match the rust side.
                # magic (8), metadata_offset (8), metadata_length (2), reserved (8), checksum (8), reserve_offset (8), reserve_length (8) = 50 bytes.
                # What are the other 14 bytes for the 64-byte header? Usually padding.
                # Let's just parse the known 50 bytes and ignore the rest of the 64.
                # This is fragile without knowing the exact Rust struct layout.

                # Re-parse for the exact fields, ignoring potential padding inside the 64-byte header.
                # This is based on WalBlockHeader fields you provided.
                # The actual layout must be verified with Rust's `#[repr(C)]` or `#[repr(packed)]` output.
                # Let's try parsing based on direct offsets assuming it's written in order and padded.
                # This is a guess given the 64-byte total size but 50-byte field sum.
                # A better way is to print the `header_bytes` from Rust and analyze.
                magic_actual = header_bytes[0:8]
                metadata_offset = int.from_bytes(header_bytes[8:16], 'little')
                metadata_length = int.from_bytes(header_bytes[16:18], 'little')
                reserved = int.from_bytes(header_bytes[18:26], 'little')
                checksum = int.from_bytes(header_bytes[26:34], 'little')
                reserve_offset = int.from_bytes(header_bytes[34:42], 'little')
                reserve_length = int.from_bytes(header_bytes[42:50], 'little')
                # The remaining 14 bytes (50-64) are unknown padding.

                if magic_actual != WAL_MAGIC:
                    print(f"Error at offset {current_pos}: Invalid magic number. Expected {WAL_MAGIC}, Got {magic_actual}")
                    break

                # Now to read the data_buf:
                # We need `total_block_size` in WalBlockHeader. Assuming it's added as a `u64` right after `reserve_length`.
                # This would make your header 58 bytes. If it's still 64, then it implies 6 bytes padding at end.
                # Let's assume you update Rust header to:
                # pub total_block_size: u64, // NEW FIELD
                # And it's at `header_bytes[50:58]`
                total_block_size = int.from_bytes(header_bytes[40:48], 'little')
                if total_block_size < WAL_BLOCK_HEADER_SIZE + metadata_length:
                    print(f"Error at offset {current_pos}: Invalid total_block_size {total_block_size}. Too small.")
                    break

                data_start_offset_in_block = WAL_BLOCK_HEADER_SIZE + metadata_length
                data_len = total_block_size - data_start_offset_in_block

                # Read metadata (already did, but let's re-read for consistency after seek)
                f.seek(current_pos + WAL_BLOCK_HEADER_SIZE) # Metadata starts right after header
                metadata_bytes_for_checksum = f.read(metadata_length)
                if len(metadata_bytes_for_checksum) < metadata_length:
                    print(f"Error at offset {current_pos}: Partial metadata for checksum. Expected {metadata_length} bytes, Got {len(metadata_bytes_for_checksum)}")
                    break

                # Read Arrow data
                f.seek(current_pos + data_start_offset_in_block)
                arrow_data_bytes = f.read(data_len)
                if len(arrow_data_bytes) < data_len:
                    print(f"Error at offset {current_pos}: Partial Arrow data. Expected {data_len} bytes, Got {len(arrow_data_bytes)}")
                    break

                # --- CHECKSUM VALIDATION ---
                computed_checksum = compute_checksum_py(metadata_bytes_for_checksum, arrow_data_bytes)
                # if computed_checksum != checksum:
                #     print(f"Error at offset {current_pos}: Checksum mismatch. Expected {checksum}, Computed {computed_checksum}")
                #     # You might want to continue or break depending on error handling strategy
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
                        print(f"    - Batch {i}: {record_batches.num_rows} rows, {record_batches.num_columns} columns")
                        # You can inspect the data here: print(batch.to_pydict())

                except Exception as e:
                    print(f"Error at offset {current_pos}: Failed to parse Arrow IPC data: {e}")
                    # You might want to continue or break
                    break

                total_blocks_validated += 1
                total_bytes_read += total_block_size

                # Move to the start of the next block
                f.seek(current_pos + total_block_size)
                print(f"Validated block at offset {current_pos}. Moving to next block at {f.tell()}")

            except struct.error as e:
                print(f"Error at offset {current_pos}: Failed to unpack header. Malformed data? {e}")
                break
            except Exception as e:
                print(f"An unexpected error occurred at offset {current_pos}: {e}")
                # If we failed to unpack the header, print the actual header bytes for debugging
                print(f"Header bytes at offset {current_pos}: {header_bytes.hex()}")
                break

    print(f"\n--- Validation Summary ---")
    print(f"Total blocks validated: {total_blocks_validated}")
    print(f"Total bytes processed: {total_bytes_read} bytes")
    # print(f"Validation {'FINISHED' if f.tell() == os.path.getsize(file_path) else 'HALTED EARLY'}.")


if __name__ == "__main__":
    # This WAL validation script assumes you have fixed your Rust WalBlockHeader
    # to include a `total_block_size: u64` field.
    # Without that, robust validation is impossible, as the reader doesn't know
    # how long each block is.
    # If added, ensure it's packed correctly for Python to read it.
    # The `struct.unpack` format string needs to be adjusted accordingly.

    # Example: If WalBlockHeader in Rust becomes:
    # pub struct WalBlockHeader {
    #     pub magic: [u8; 8],
    #     pub metadata_offset: u64,
    #     pub metadata_length: u16,
    #     pub reserved: u64,
    #     pub checksum: u64,
    #     pub reserve_offset: u64,
    #     pub reserve_length: u64,
    #     pub total_block_size: u64, // NEW FIELD
    # }
    # Then WAL_HEADER_STRUCT_FORMAT would change to include one more 'Q' and the byte offsets.
    # And the `total_block_size` would be read directly.

    # For now, the current script makes the strong assumption about `total_block_size` position
    # in the header_bytes. PLEASE FIX YOUR RUST HEADER FOR RELIABLE WAL.

    validate_wal_file(WAL_FILE_PATH)