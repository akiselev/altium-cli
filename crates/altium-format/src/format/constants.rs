//! Magic numbers and binary format constants.

/// Mask for extracting size from block header (clears flag bits in high byte).
pub const SIZE_FLAG_MASK: u32 = 0x00FFFFFF;

/// Flag indicating binary record format (in block header flags byte).
pub const BLOCK_FLAG_BINARY: u8 = 0x01;

/// Tag byte for zlib-compressed CFB storage streams.
pub const CFB_COMPRESSED_TAG: u8 = 0xD0;
