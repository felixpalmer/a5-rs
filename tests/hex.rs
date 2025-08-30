use a5::core::hex::{big_int_to_hex, hex_to_big_int};

#[test]
fn test_hex_conversion() {
    let hex = "1a2b3c4d";
    let big_int = hex_to_big_int(hex);
    let result = big_int_to_hex(&big_int);
    assert_eq!(result, hex);
}

#[test]
fn test_hex_conversion_with_zero() {
    let hex = "0";
    let big_int = hex_to_big_int(hex);
    let result = big_int_to_hex(&big_int);
    assert_eq!(result, hex);
}

#[test]
fn test_hex_conversion_with_large_number() {
    let hex = "ffffffffffffffff";
    let big_int = hex_to_big_int(hex);
    let result = big_int_to_hex(&big_int);
    assert_eq!(result, hex);
}
