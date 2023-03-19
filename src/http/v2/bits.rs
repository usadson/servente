// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub(super) const fn convert_be_u24_to_u32(bytes: [u8; 3]) -> u32 {
    u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]])
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::convert_be_u24_to_u32;

    #[rstest]
    #[case([0x00, 0x00, 0xFF], 0xFF)]
    #[case([0x00, 0xFF, 0x00], 0xFF00)]
    #[case([0x00, 0xFF, 0xAA], 0xFFAA)]
    #[case([0xFF, 0x00, 0x00], 0xFF0000)]
    #[case([0xFF, 0xFF, 0xFF], 0xFFFFFF)]
    #[case([0xAA, 0xBB, 0xCC], 0xAABBCC)]
    fn test_convert_be_u24_to_u32(#[case] input: [u8; 3], #[case] expected: u32) {
        // println!("res={:x}\texpected={:x}",
        assert_eq!(
            convert_be_u24_to_u32(input), expected);
    }
}
