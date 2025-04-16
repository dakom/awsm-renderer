pub fn slice_zeroes(size: usize) -> &'static [u8] {
    static ZEROES_1: [u8; 1] = [0; 1];
    static ZEROES_2: [u8; 2] = [0; 2];
    static ZEROES_3: [u8; 3] = [0; 3];
    static ZEROES_4: [u8; 4] = [0; 4];
    static ZEROES_5: [u8; 5] = [0; 5];
    static ZEROES_6: [u8; 6] = [0; 6];
    static ZEROES_7: [u8; 7] = [0; 7];
    static ZEROES_8: [u8; 8] = [0; 8];
    static ZEROES_9: [u8; 9] = [0; 9];
    static ZEROES_10: [u8; 10] = [0; 10];
    static ZEROES_11: [u8; 11] = [0; 11];
    static ZEROES_12: [u8; 12] = [0; 12];
    static ZEROES_13: [u8; 13] = [0; 13];
    static ZEROES_14: [u8; 14] = [0; 14];
    static ZEROES_15: [u8; 15] = [0; 15];
    static ZEROES_16: [u8; 16] = [0; 16];
    static ZEROES_17: [u8; 17] = [0; 17];
    static ZEROES_18: [u8; 18] = [0; 18];
    static ZEROES_19: [u8; 19] = [0; 19];
    static ZEROES_20: [u8; 20] = [0; 20];
    static ZEROES_21: [u8; 21] = [0; 21];
    static ZEROES_22: [u8; 22] = [0; 22];
    static ZEROES_23: [u8; 23] = [0; 23];
    static ZEROES_24: [u8; 24] = [0; 24];
    static ZEROES_25: [u8; 25] = [0; 25];
    static ZEROES_26: [u8; 26] = [0; 26];
    static ZEROES_27: [u8; 27] = [0; 27];
    static ZEROES_28: [u8; 28] = [0; 28];
    static ZEROES_29: [u8; 29] = [0; 29];
    static ZEROES_30: [u8; 30] = [0; 30];
    static ZEROES_31: [u8; 31] = [0; 31];
    static ZEROES_32: [u8; 32] = [0; 32];

    match size {
        1 => &ZEROES_1,
        2 => &ZEROES_2,
        3 => &ZEROES_3,
        4 => &ZEROES_4,
        5 => &ZEROES_5,
        6 => &ZEROES_6,
        7 => &ZEROES_7,
        8 => &ZEROES_8,
        9 => &ZEROES_9,
        10 => &ZEROES_10,
        11 => &ZEROES_11,
        12 => &ZEROES_12,
        13 => &ZEROES_13,
        14 => &ZEROES_14,
        15 => &ZEROES_15,
        16 => &ZEROES_16,
        17 => &ZEROES_17,
        18 => &ZEROES_18,
        19 => &ZEROES_19,
        20 => &ZEROES_20,
        21 => &ZEROES_21,
        22 => &ZEROES_22,
        23 => &ZEROES_23,
        24 => &ZEROES_24,
        25 => &ZEROES_25,
        26 => &ZEROES_26,
        27 => &ZEROES_27,
        28 => &ZEROES_28,
        29 => &ZEROES_29,
        30 => &ZEROES_30,
        31 => &ZEROES_31,
        32 => &ZEROES_32,
        _ => panic!("Invalid size for zeroes slice: {}", size),
    }
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