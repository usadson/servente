// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub const FILE_JPEG_MAGIC_NUMBER: &[u8; 3] = &[0xFF, 0xD8, 0xFF];
pub const FILE_PNG_MAGIC_NUMBER: &[u8; 8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
