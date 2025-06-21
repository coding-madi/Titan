use crate::actors::broadcast::RecordBatchWrapper;
use crate::utils::cksum;
use crate::wal::layout::WalBlockHeader;
use arrow_ipc::writer::StreamWriter;
use bytemuck::bytes_of;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::SeekFrom;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

// Pre-allocate a file to a specific size
pub async fn pre_allocate_file(file_path: &str, size: u64) -> std::io::Result<()> {
    let file = File::create(file_path)?;
    file.set_len(size)?;
    Ok(())
}

pub async fn write_wal_block_async(
    path: &str,
    offset: u64,
    record_batch_wrapper: Arc<RecordBatchWrapper>,
    metadata: &[u8; 48],
) -> std::io::Result<()> {
    // Serialize Arrow RecordBatch in a blocking task
    let (data_buf, _schema) = tokio::task::spawn_blocking({
        let wrapper = Arc::clone(&record_batch_wrapper);
        move || {
            let record_batch = wrapper.data.get(0).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "RecordBatchWrapper does not contain any RecordBatch",
                )
            })?;

            let mut data_buf = Vec::new();
            let mut arrow_writer =
                StreamWriter::try_new(&mut data_buf, &record_batch.schema()).unwrap();
            arrow_writer.write(record_batch).unwrap();
            arrow_writer.finish().unwrap();
            Ok::<_, std::io::Error>((data_buf, record_batch.schema()))
        }
    })
    .await
    .unwrap()?; // unwrap join error, ? on result

    let metadata_offset = 48;
    let metadata_length = metadata.len() as u16;

    let reserve_offset = metadata_offset + metadata.len();
    let reserve_length = 0;

    let checksum = cksum::compute_checksum(metadata, &data_buf);

    // --- CRITICAL CHANGE HERE: Open a new file handle for this specific write ---
    let mut file = OpenOptions::new()
        .write(true)
        .open(&*path) // Deref Arc<String> to &str
        .await?;

    let header = WalBlockHeader {
        magic: *b"WALBLOCK",
        metadata_offset: metadata_offset as u64,
        metadata_length,
        reserved: 0,
        checksum,
        reserve_offset: reserve_offset as u64,
        reserve_length: reserve_length as u64,
        total_block_size: (std::mem::size_of::<WalBlockHeader>()
            + metadata.len()
            + data_buf.len()) as u64,
    };

    file.seek(SeekFrom::Start(offset)).await?;
    // Write to file in async context
    let header_bytes = bytes_of(&header);
    file.write_all(header_bytes).await?;
    file.write_all(metadata).await?;
    file.write_all(&data_buf).await?;
    file.flush().await?;
    Ok(())
}

pub fn write_wal_block(
    writer: &mut BufWriter<File>,
    record_batch_wrapper: &RecordBatchWrapper,
    metadata: &[u8; 48],
) -> std::io::Result<()> {
    // Serialize Arrow RecordBatch into buffer
    let mut data_buf = Vec::new();
    let record_batch = record_batch_wrapper.data.get(0).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "RecordBatchWrapper does not contain any RecordBatch",
        )
    })?;
    {
        let mut arrow_writer =
            StreamWriter::try_new(&mut data_buf, &record_batch.schema()).unwrap();
        arrow_writer.write(record_batch).unwrap();
        arrow_writer.finish().unwrap();
    }

    let metadata_offset = 48; // right after header
    let metadata_length = metadata.len() as u16;

    let reserve_offset = metadata_offset + metadata.len();
    let reserve_length = 0; // we\u2019re not using reserve now

    let checksum = cksum::compute_checksum(&metadata, &data_buf);

    // Construct the header
    let header = WalBlockHeader {
        magic: *b"WALBLOCK",
        metadata_offset: metadata_offset as u64,
        metadata_length,
        reserved: 0,
        checksum,
        reserve_offset: reserve_offset as u64,
        reserve_length: reserve_length as u64,
        total_block_size: (std::mem::size_of::<WalBlockHeader>()
            + metadata.len()
            + data_buf.len()) as u64,
    };

    // Write header
    // let header_bytes: &[u8; std::mem::size_of::<WalBlockHeader>()] =
    //     unsafe { std::mem::transmute(&header) };
    // writer.write_all(header_bytes)?;

    let header_bytes = bytes_of(&header);
    writer.write_all(header_bytes)?;

    // Write metadata
    writer.write_all(metadata)?;

    // Write data
    writer.write_all(&data_buf)?;
    // Optional: write reserve/padding here
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Int32Array, RecordBatch};
    use arrow_schema::{Field, Schema};
    use std::io::{BufWriter, Read, Seek, SeekFrom};
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_and_validate_wal_block() -> std::io::Result<()> {
        // Step 1: Create temporary file
        let temp_file = NamedTempFile::new()?;
        let file_path = temp_file.path().to_path_buf();
        let mut writer = BufWriter::new(temp_file.reopen()?);

        // Step 2: Create dummy Arrow RecordBatch
        let schema = Arc::new(Schema::new(vec![Field::new(
            "value",
            arrow_schema::DataType::Int32,
            false,
        )]));

        let array = Arc::new(Int32Array::from(vec![10, 20, 30, 40, 50]));
        let batch = RecordBatch::try_new(schema.clone(), vec![array])
            .expect("Failed to create RecordBatch");

        // Step 3: Metadata as raw bytes (could be FlatBuf later)
        let metadata = b"{\"wal\": \"test-block\"}";

        // Step 4: Write WAL block
        write_wal_block(&mut writer, &batch, metadata)?;
        writer.flush()?; // Ensure it's written to disk

        // Step 5: Read back the header for validation
        let mut file = std::fs::File::open(file_path)?;
        let mut header_buf = [0u8; std::mem::size_of::<WalBlockHeader>()];
        file.read_exact(&mut header_buf)?;

        let header: WalBlockHeader =
            unsafe { std::ptr::read_unaligned(header_buf.as_ptr() as *const _) };

        // Step 6: Assertions
        assert_eq!(&header.magic, b"WALBLOCK");
        assert_eq!(header.metadata_length, metadata.len() as u16);
        assert!(header.checksum != 0); // Should be valid CRC

        // Optionally: Read metadata back and assert
        file.seek(SeekFrom::Start(header.metadata_offset))?;
        let mut meta_buf = vec![0u8; header.metadata_length as usize];
        file.read_exact(&mut meta_buf)?;
        assert_eq!(meta_buf, metadata);

        Ok(())
    }
}
