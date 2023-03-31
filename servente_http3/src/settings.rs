// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum SettingKind {
    QpackMaxTableCapacity = 0x01,
    MaxFieldSectionSize = 0x06,
    QpackBlockedStreams = 0x07,
    EnableConnectProtocol = 0x08,
    H3Datagram = 0x33,
    EnableMetadata = 0x4d44,
}

impl TryFrom<usize> for SettingKind {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, ()> {
        Ok(match value {
            0x01 => Self::QpackMaxTableCapacity,
            0x06 => Self::MaxFieldSectionSize,
            0x07 => Self::QpackBlockedStreams,
            0x08 => Self::EnableConnectProtocol,
            0x33 => Self::H3Datagram,
            0x4d44 => Self::EnableMetadata,
            _ => return Err(()),
        })
    }
}
