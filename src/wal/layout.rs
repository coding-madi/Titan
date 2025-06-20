use arrow_array::RecordBatch;
use bytemuck::{Pod, Zeroable};

use crate::schema::wal_schema_generated::wal::FlatbufMeta;

#[derive(Debug, Clone)]
pub enum MetadataData<'a> {
    Flatbuf(FlatbufMeta<'a>),
    Unknown(Vec<u8>), // Forward-compatible blob
}

#[repr(C, align(32))]
#[derive(Debug, Clone, Copy)]
pub struct WalBlockHeader {
    pub magic: [u8; 8],       // 8 bytes
    pub metadata_offset: u64, // 8 bytes
    pub metadata_length: u16, // 2 bytes
    pub reserved: u16,        // 2 bytes (padding)
    pub checksum: u32,        // 4 bytes
    pub reserve_offset: u64,  // 8 bytes
    pub reserve_length: u64,  // 8 bytes
                              // total: 40 bytes → padded to 64 for SIMD
}

// Required by bytemuck
unsafe impl Zeroable for WalBlockHeader {}
unsafe impl Pod for WalBlockHeader {}

#[derive(Debug, Clone)]
pub struct Metadata<'a> {
    pub data: MetadataData<'a>,
    pub offset: u64,   // Offset in the WAL file
    pub length: u16,   // Length of the metadata in bytes
    pub checksum: u32, // Checksum of the metadata
}

#[derive(Debug, Clone)]
pub struct WalBlock<'a> {
    pub header: WalBlockHeader,
    pub metadata: Metadata<'a>,
    pub data: RecordBatch,
}

#[derive(Debug, Clone)]
pub struct Reserve {
    pub offset: u64,
    pub length: u64,
}
