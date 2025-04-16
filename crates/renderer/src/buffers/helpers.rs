pub fn slice_zeroes(size: usize) -> &'static [u8] {
    const ZEROES: [u8; 32] = [0; 32];
    &ZEROES[..size]
}

#[allow(dead_code)]
pub fn debug_chunks_to_f32(slice: &[u8], chunk_size: usize) -> Vec<Vec<f32>> {
    debug_slice_to_f32(slice)
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

#[allow(dead_code)]
pub fn debug_slice_to_f32(slice: &[u8]) -> Vec<f32> {
    let mut f32s = Vec::new();
    for i in (0..slice.len()).step_by(4) {
        let bytes = &slice[i..i + 4];
        let f32_value = f32::from_le_bytes(bytes.try_into().unwrap());
        f32s.push(f32_value);
    }
    f32s
}

#[allow(dead_code)]
pub fn debug_slice_to_u16(slice: &[u8]) -> Vec<u16> {
    let mut u16s = Vec::new();
    for i in (0..slice.len()).step_by(2) {
        let bytes = &slice[i..i + 2];
        let u16_value = u16::from_le_bytes(bytes.try_into().unwrap());
        u16s.push(u16_value);
    }
    u16s
}

#[allow(dead_code)]
pub fn debug_slice_to_u32(slice: &[u8]) -> Vec<u32> {
    let mut u32s = Vec::new();
    for i in (0..slice.len()).step_by(4) {
        let bytes = &slice[i..i + 4];
        let u32_value = u32::from_le_bytes(bytes.try_into().unwrap());
        u32s.push(u32_value);
    }
    u32s
}