use lz4::liblz4::BlockChecksum;
use std::io;

pub fn compress(input: &[u8]) -> io::Result<Vec<u8>> {
    let buf = Vec::with_capacity(input.len() / 2);
    let mut encoder = lz4::EncoderBuilder::new()
        .block_checksum(BlockChecksum::NoBlockChecksum)
        .build(buf)?;
    let mut reader = io::Cursor::new(input);
    io::copy(&mut reader, &mut encoder)?;
    let (bytes, result) = encoder.finish();
    result?;
    Ok(bytes)
}

pub fn decompress(compressed_bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(10 * compressed_bytes.len());
    let reader = io::Cursor::new(compressed_bytes);
    let mut decoder = lz4::Decoder::new(reader)?;
    io::copy(&mut decoder, &mut buf)?;
    let (_, result) = decoder.finish();
    result?;
    Ok(buf)
}
