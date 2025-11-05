//! SIMD optimizations module
//!
//! This module contains all unsafe SIMD code isolated in dedicated submodules.

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
pub mod neon;
