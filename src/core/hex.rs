// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

/// Converts a hexadecimal string to a u64
///
/// # Arguments
///
/// * `hex` - A string containing a hexadecimal number
///
/// # Returns
///
/// A u64 representing the hexadecimal value
pub fn hex_to_u64(hex: &str) -> Result<u64, String> {
    // Remove any "0x" prefix if present
    let hex = hex.trim_start_matches("0x");
    u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex string: {}", e))
}

/// Converts a u64 to a hexadecimal string
///
/// # Arguments
///
/// * `value` - A u64 to convert
///
/// # Returns
///
/// A string containing the hexadecimal representation
pub fn u64_to_hex(value: u64) -> String {
    format!("{value:x}")
}
