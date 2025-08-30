use a5::core::hex::{hex_to_u64, u64_to_hex};

#[test]
fn test_hex_conversion() {
    let hex = "1a2b3c4d";
    let u64_val = hex_to_u64(hex).unwrap();
    let result = u64_to_hex(u64_val);
    assert_eq!(result, hex);
}

#[test]
fn test_hex_conversion_with_zero() {
    let hex = "0";
    let u64_val = hex_to_u64(hex).unwrap();
    let result = u64_to_hex(u64_val);
    assert_eq!(result, hex);
}

#[test]
fn test_hex_conversion_with_large_number() {
    let hex = "ffffffffffffffff";
    let u64_val = hex_to_u64(hex).unwrap();
    let result = u64_to_hex(u64_val);
    assert_eq!(result, hex);
}
