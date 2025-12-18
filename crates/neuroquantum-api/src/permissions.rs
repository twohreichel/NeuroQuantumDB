//! Permission constants and utilities for NeuroQuantumDB API
//!
//! This module provides static string constants for permissions to avoid
//! repeated heap allocations when using permission strings throughout the codebase.
//!
//! # Usage
//!
//! ```rust,ignore
//! use neuroquantum_api::permissions::{Permission, ADMIN, READ, WRITE};
//!
//! // Using constants directly
//! let perms = Permission::admin_permissions();
//!
//! // Or using individual constants
//! let read_only = vec![READ.to_string()];
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
    pub fn read_only() -> Vec<String> {
        vec![READ.to_string()]
    }

    /// Returns read-write permissions as owned String vec.
    #[inline]
    pub fn read_write() -> Vec<String> {
        vec![READ.to_string(), WRITE.to_string()]
    }

    /// Returns neuromorphic read permissions as owned String vec.
    #[inline]
    pub fn neuromorphic_read() -> Vec<String> {
        vec![NEUROMORPHIC.to_string(), READ.to_string()]
    }

    /// Returns quantum read permissions as owned String vec.
    #[inline]
    pub fn quantum_read() -> Vec<String> {
        vec![QUANTUM.to_string(), READ.to_string()]
    }

    /// Returns DNA read-write permissions as owned String vec.
    #[inline]
    pub fn dna_read_write() -> Vec<String> {
        vec![DNA.to_string(), READ.to_string(), WRITE.to_string()]
    }

    /// Returns quantum authenticated permission as owned String vec.
    #[inline]
    pub fn quantum_authenticated() -> Vec<String> {
        vec![QUANTUM_AUTHENTICATED.to_string()]
    }

    /// Convert a slice of static permission strings to owned Strings.
    ///
    /// Useful when you have a list of permission constants and need `Vec<String>`.
    #[inline]
    pub fn to_owned(permissions: &[&str]) -> Vec<String> {
        permissions.iter().map(|s| (*s).to_string()).collect()
    }

    /// Check if a permission string matches a known permission constant.
    #[inline]
    pub fn is_valid(permission: &str) -> bool {
        ALL_PERMISSIONS.contains(&permission)
    }

    /// Check if the given permissions include admin.
    #[inline]
    pub fn has_admin(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == ADMIN)
    }

    /// Check if the given permissions include read.
    #[inline]
    pub fn has_read(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == READ)
    }

    /// Check if the given permissions include write.
    #[inline]
    pub fn has_write(permissions: &[String]) -> bool {
        permissions.iter().any(|p| p == WRITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_permissions() {
        let perms = Permission::admin_permissions();
        assert_eq!(perms.len(), 6);
        assert!(perms.contains(&ADMIN.to_string()));
        assert!(perms.contains(&READ.to_string()));
        assert!(perms.contains(&WRITE.to_string()));
    }

    #[test]
    fn test_read_only() {
        let perms = Permission::read_only();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0], READ);
    }

    #[test]
    fn test_read_write() {
        let perms = Permission::read_write();
        assert_eq!(perms.len(), 2);
        assert!(perms.contains(&READ.to_string()));
        assert!(perms.contains(&WRITE.to_string()));
    }

    #[test]
    fn test_is_valid() {
        assert!(Permission::is_valid(ADMIN));
        assert!(Permission::is_valid(READ));
        assert!(Permission::is_valid("quantum"));
        assert!(!Permission::is_valid("invalid_permission"));
    }

    #[test]
    fn test_has_admin() {
        let admin_perms = Permission::admin_permissions();
        let read_perms = Permission::read_only();

        assert!(Permission::has_admin(&admin_perms));
        assert!(!Permission::has_admin(&read_perms));
    }

    #[test]
    fn test_to_owned() {
        let perms = Permission::to_owned(&[READ, WRITE]);
        assert_eq!(perms.len(), 2);
        assert_eq!(perms[0], "read");
        assert_eq!(perms[1], "write");
    }
}
