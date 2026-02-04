// Compute checksum of metadata + data

pub fn compute_checksum(metadata: &Vec<u8>, data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(metadata);
    hasher.update(data);
    hasher.finalize()
}
