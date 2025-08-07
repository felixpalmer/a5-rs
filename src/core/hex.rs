// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use num_bigint::BigInt;
use num_traits::Num;

/// Converts a hexadecimal string to a BigInt
///
/// # Arguments
///
/// * `hex` - A string containing a hexadecimal number
///
/// # Returns
///
/// A BigInt representing the hexadecimal value
pub fn hex_to_big_int(hex: &str) -> BigInt {
    // Remove any "0x" prefix if present
    let hex = hex.trim_start_matches("0x");
    BigInt::from_str_radix(hex, 16).expect("Invalid hex string")
}

/// Converts a BigInt to a hexadecimal string
///
/// # Arguments
///
/// * `value` - A BigInt to convert
///
/// # Returns
///
/// A string containing the hexadecimal representation
pub fn big_int_to_hex(value: &BigInt) -> String {
    format!("{:x}", value)
}
