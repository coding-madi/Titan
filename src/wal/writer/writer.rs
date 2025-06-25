use crate::actors::broadcast_actor::RecordBatchWrapper;
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
    metadata: &Vec<u8>,
) -> std::io::Result<()> {
    // Serialize Arrow RecordBatch in a blocking task
    let (data_buf, _schema) = tokio::task::spawn_blocking({
        let wrapper = Arc::clone(&record_batch_wrapper);
        move || {
            let record_batch = &wrapper.data;

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

    let metadata_offset = 63;
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
        checksum,
        reserve_offset: reserve_offset as u64,
        data_offset: metadata_offset as u64 + metadata_length as u64,
        data_length: data_buf.len() as u64,
        padding: 0, // Padding to align to 64 bytes, can be set later if needed
        reserve_length: reserve_length as u64,
        total_block_size: (std::mem::size_of::<WalBlockHeader>() + metadata.len() + data_buf.len())
            as u64,
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
    data_buf: &Vec<u8>,
    metadata: &Vec<u8>,
) -> std::io::Result<()> {
    // Serialize Arrow RecordBatch into buffer

    let metadata_offset = 63; // right after header
    let metadata_length = metadata.len() as u16;

    let reserve_offset = metadata_offset + metadata.len();
    let reserve_length = 0; // we\u2019re not using reserve now

    let checksum = cksum::compute_checksum(metadata, data_buf);

    // pub magic: [u8; 8],        // 8 bytes
    // pub metadata_offset: u64,  // 8 bytes
    // pub metadata_length: u16,  // 2 bytes Size of the Flatbuf metadata
    // pub checksum: u32,         // 4 bytes
    // pub reserve_offset: u64,   // 8 bytes
    // pub reserve_length: u64,   // 8 bytes
    // pub data_offset: u64,      // 8 bytes
    // pub data_length: u64,      // 8 bytes
    // pub total_block_size: u64, // 8 bytes Total size of the block in bytes
    // pub padding: u8, // 8 bytes Padding to align to 64 bytes
    // Construct the header
    let header = WalBlockHeader {
        magic: *b"WALBLOCK",
        metadata_offset: metadata_offset as u64,
        metadata_length,
        checksum,
        reserve_offset: reserve_offset as u64,
        data_offset: metadata_offset as u64 + metadata_length as u64,
        data_length: data_buf.len() as u64,
        padding: 0, // Padding to align to 64 bytes, can be set later if needed
        reserve_length: reserve_length as u64,
        total_block_size: (std::mem::size_of::<WalBlockHeader>() + metadata.len() + data_buf.len())
            as u64,
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
