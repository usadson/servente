// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module contains file system utilities.

/// The `unix` module contains utilities for working with POSIX/*NIX, but is
/// named *unix* to follow convention in the [`std::os`] Rust standard.
pub mod unix {

    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct CategoryPermissions {
        read: bool,
        write: bool,
        execute: bool,
    }

    impl CategoryPermissions {
        /// Constructs this object from the lower three bits of the given byte.
        const fn from_bits(b: u8) -> Self {
            CategoryPermissions {
                read: (b >> 2) & 0b1 == 1,
                write: (b >> 1) & 0b1 == 1,
                execute: b & 0b1 == 1
            }
        }

        /// Is the file executable for this mode category.
        pub const fn is_executable(&self) -> bool {
            self.execute
        }

        /// Is the file readable for this mode category.
        pub const fn is_readable(&self) -> bool {
            self.read
        }

        /// Is the file writable for this mode category.
        pub const fn is_writable(&self) -> bool {
            self.write
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub enum PermissionCategory {
        Owner,
        Group,
        Others,
    }

    impl PermissionCategory {
        pub const fn all() -> &'static [PermissionCategory] {
            &[
                PermissionCategory::Owner,
                PermissionCategory::Group,
                PermissionCategory::Others
            ]
        }
    }

    /// A wrapper around the [`mode_t`]() type of POSIX.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct UnixFileMode(u32);

    impl UnixFileMode {
        pub const fn new(mode_t: u32) -> Self {
            Self(mode_t)
        }

        pub const fn inner(&self) -> u32 {
            self.0
        }

        pub const fn permisions(&self, category: PermissionCategory) -> CategoryPermissions {
            match category {
                PermissionCategory::Owner => CategoryPermissions::from_bits((self.0 as u8 >> 6) & 0b111),
                PermissionCategory::Group => CategoryPermissions::from_bits((self.0 as u8 >> 3) & 0b111),
                PermissionCategory::Others => CategoryPermissions::from_bits(self.0 as u8 & 0b111),
            }
        }
    }

}

pub trait PermissionsExt {
    fn is_executable(&self) -> bool;

    #[cfg(unix)]
    fn mode_t(&self) -> unix::UnixFileMode;
}

#[cfg(unix)]
impl<T> PermissionsExt for T
        where T: std::os::unix::fs::PermissionsExt {

    fn is_executable(&self) -> bool {
        let mode = self.mode_t();

        unix::PermissionCategory::all()
            .iter()
            .any(|category| mode.permisions(*category).is_executable())
    }

    fn mode_t(&self) -> unix::UnixFileMode {
        unix::UnixFileMode::new(self.mode())
    }
}
