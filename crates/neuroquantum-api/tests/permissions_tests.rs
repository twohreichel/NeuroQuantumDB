//! Tests for permission handling
//!
//! These tests validate permission creation, validation, and helper methods.

use neuroquantum_api::permissions::{Permission, ADMIN, READ, WRITE};

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
