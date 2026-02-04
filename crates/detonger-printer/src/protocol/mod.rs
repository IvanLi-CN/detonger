//! Protocol layer: encode Detonger printer jobs and split them into vendor messages.
//!
//! This module is intentionally transport-agnostic: it only deals with bytes.

pub mod encode;
pub mod split;

pub use encode::{FinalizeMode, encode_png_job_messages, encode_width_test_job_messages};
pub use split::split_vendor_messages;
