//! Permission constants and utilities for `NeuroQuantumDB` API
//!
//! This module provides static string constants for permissions to avoid
//! repeated heap allocations when using permission strings throughout the codebase.
//!
//! # Usage
//!
//! ```rust
//! use neuroquantum_api::permissions::{Permission, ADMIN, READ, WRITE};
//!
//! // Using constants directly
//! let perms = Permission::admin_permissions();
//! assert!(perms.contains(&ADMIN.to_string()));
//!
//! // Or using individual constants
//! let read_only = vec![READ.to_string()];
//! assert_eq!(read_only.len(), 1);
//! ```

/// Permission for administrative operations
pub const ADMIN: &str = "admin";

/// Permission for neuromorphic operations
pub const NEUROMORPHIC: &str = "neuromorphic";

/// Permission for quantum operations
pub const QUANTUM: &str = "quantum";

/// Permission for DNA storage operations
pub const DNA: &str = "dna";

/// Permission for read operations
pub const READ: &str = "read";

/// Permission for write operations
pub const WRITE: &str = "write";

/// Permission granted after quantum authentication
pub const QUANTUM_AUTHENTICATED: &str = "quantum_authenticated";

/// All available permissions as static strings
pub const ALL_PERMISSIONS: &[&str] = &[ADMIN, NEUROMORPHIC, QUANTUM, DNA, READ, WRITE];

/// Permission utilities
pub struct Permission;

impl Permission {
    /// Returns all admin permissions as owned Strings.
    ///
    /// This allocates strings but is intended for cases where `Vec<String>` is required
    /// (e.g., serialization, API boundaries).
    #[inline]
    #[must_use]
    pub fn admin_permissions() -> Vec<String> {
        vec![
            ADMIN.to_string(),
            NEUROMORPHIC.to_string(),
            QUANTUM.to_string(),
            DNA.to_string(),
            READ.to_string(),
            WRITE.to_string(),
        ]
    }

    /// Returns read-only permission as owned String vec.
    #[inline]
    #[must_use]
    pub fn read_only() -> Vec<String> {
        vec![READ.to_string()]
    }

    /// Returns read-write permissions as owned String vec.
    #[inline]
    #[must_use]
    pub fn read_write() -> Vec<String> {
        vec![READ.to_string(), WRITE.to_string()]
    }

    /// Returns neuromorphic read permissions as owned String vec.
    #[inline]
    #[must_use]
    pub fn neuromorphic_read() -> Vec<String> {
        vec![NEUROMORPHIC.to_string(), READ.to_string()]
    }

    /// Returns quantum read permissions as owned String vec.
    #[inline]
    #[must_use]
    pub fn quantum_read() -> Vec<String> {
        vec![QUANTUM.to_string(), READ.to_string()]
    }

    /// Returns DNA read-write permissions as owned String vec.
    #[inline]
    #[must_use]
    pub fn dna_read_write() -> Vec<String> {
        vec![DNA.to_string(), READ.to_string(), WRITE.to_string()]
    }

    /// Returns quantum authenticated permission as owned String vec.
    #[inline]
    #[must_use]
    pub fn quantum_authenticated() -> Vec<String> {
        vec![QUANTUM_AUTHENTICATED.to_string()]
    }

    /// Convert a slice of static permission strings to owned Strings.
    ///
    /// Useful when you have a list of permission constants and need `Vec<String>`.
    #[inline]
    #[must_use]
    pub fn to_owned(permissions: &[&str]) -> Vec<String> {
        permissions.iter().map(|s| (*s).to_string()).collect()
    }

    /// Check if a permission string matches a known permission constant.
    #[inline]
    #[must_use]
    pub fn is_valid(permission: &str) -> bool {
        ALL_PERMISSIONS.contains(&permission)
    }

    /// Check if the given permissions include admin.
    #[inline]
    #[must_use]
    pub fn has_admin(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == ADMIN)
    }

    /// Check if the given permissions include read.
    #[inline]
    #[must_use]
    pub fn has_read(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == READ)
    }

    /// Check if the given permissions include write.
    #[inline]
    #[must_use]
    pub fn has_write(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == WRITE)
    }
}
