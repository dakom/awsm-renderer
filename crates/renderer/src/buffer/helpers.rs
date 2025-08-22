pub fn slice_zeroes(size: usize) -> &'static [u8] {
    const ZEROES: [u8; 32] = [0; 32];
    &ZEROES[..size]
}

#[allow(dead_code)]
pub fn debug_chunks_to_f32(slice: &[u8], chunk_size: usize) -> Vec<Vec<f32>> {
    u8_to_f32_vec(slice)
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// From gltf spec:
// "All buffer data defined in this specification (i.e., geometry attributes, geometry indices, sparse accessor data, animation inputs and outputs, inverse bind matrices)
// MUST use little endian byte order."

pub fn u16_to_u32_vec(v: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(v.len() * 2);
    for chunk in v.chunks_exact(2) {
        let value = u16::from_le_bytes([chunk[0], chunk[1]]);
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

pub fn i16_to_i32_vec(v: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(v.len() * 2);
    for chunk in v.chunks_exact(2) {
        let value = i16::from_le_bytes([chunk[0], chunk[1]]);
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

pub fn u8_to_f32_vec(v: &[u8]) -> Vec<f32> {
    v.chunks_exact(4)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(f32::from_le_bytes)
        .collect()
}

pub fn u8_to_f32_iter(v: &[u8]) -> impl Iterator<Item = f32> + '_ {
    v.chunks_exact(4)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(f32::from_le_bytes)
}

pub fn u8_to_i8_vec(v: &[u8]) -> Vec<i8> {
    v.chunks_exact(1)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(i8::from_le_bytes)
        .collect()
}

pub fn u8_to_u16_vec(v: &[u8]) -> Vec<u16> {
    v.chunks_exact(2)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(u16::from_le_bytes)
        .collect()
}

pub fn u8_to_u16_iter(v: &[u8]) -> impl Iterator<Item = u16> + '_ {
    v.chunks_exact(2)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(u16::from_le_bytes)
}

pub fn u8_to_i16_vec(v: &[u8]) -> Vec<i16> {
    v.chunks_exact(2)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(i16::from_le_bytes)
        .collect()
}

pub fn u8_to_u32_vec(v: &[u8]) -> Vec<u32> {
    v.chunks_exact(4)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(u32::from_le_bytes)
        .collect()
}

pub fn u8_to_u32_iter(v: &[u8]) -> impl Iterator<Item = u32> + '_ {
    v.chunks_exact(4)
        .map(TryInto::try_into)
        .map(Result::unwrap)
        .map(u32::from_le_bytes)
}
