use num_bigint::BigUint;
use obfuskey::{alphabets, Obfuskey, ObfuskeyError};

// =============================================================================
// Encode/Decode tests — must match Python/JS exactly
// =============================================================================

#[test]
fn test_encode_u128() {
    use obfuskey::Alphabet;
    use obfuskey::encode::encode_u128;

    let alpha = Alphabet::new(alphabets::BASE62).unwrap();

    assert_eq!(encode_u128(10000, &alpha), "2bI");
    assert_eq!(encode_u128(50000, &alpha), "D0S");
    assert_eq!(encode_u128(0, &alpha), "0");
    assert_eq!(encode_u128(1, &alpha), "1");
    assert_eq!(encode_u128(61, &alpha), "z");
    assert_eq!(encode_u128(62, &alpha), "10");
    assert_eq!(encode_u128(56800235583, &alpha), "zzzzzz"); // 62^6 - 1
}

#[test]
fn test_decode_u128() {
    use obfuskey::Alphabet;
    use obfuskey::decode::decode_u128;

    let alpha = Alphabet::new(alphabets::BASE62).unwrap();

    assert_eq!(decode_u128("2bI", &alpha).unwrap(), Some(10000));
    assert_eq!(decode_u128("D0S", &alpha).unwrap(), Some(50000));
    assert_eq!(decode_u128("0", &alpha).unwrap(), Some(0));
    assert_eq!(decode_u128("10", &alpha).unwrap(), Some(62));
    assert_eq!(decode_u128("zzzzzz", &alpha).unwrap(), Some(56800235583));
}

#[test]
fn test_encode_decode_big() {
    use obfuskey::Alphabet;
    use obfuskey::decode::decode_big;
    use obfuskey::encode::encode_big;

    let alpha = Alphabet::new(alphabets::BASE62).unwrap();

    assert_eq!(encode_big(&BigUint::from(10000u64), &alpha), "2bI");
    assert_eq!(decode_big("2bI", &alpha).unwrap(), BigUint::from(10000u64));
    assert_eq!(decode_big("zzzzzz", &alpha).unwrap(), BigUint::from(56800235583u64));
}

// =============================================================================
// Prime generation tests — must match Python/JS exactly
// =============================================================================

#[test]
fn test_generate_prime_base36_8() {
    let result = obfuskey::math::generate_prime(36, 8, 1.618033988749894848).unwrap();
    assert_eq!(result, BigUint::from(4564651716269u64));
}

#[test]
fn test_generate_prime_base36_8_175() {
    let result = obfuskey::math::generate_prime(36, 8, 1.75).unwrap();
    assert_eq!(result, BigUint::from(4936942338091u64));
}

#[test]
fn test_generate_prime_base16_4() {
    let result = obfuskey::math::generate_prime(16, 4, 1.618033988749894848).unwrap();
    assert_eq!(result, BigUint::from(106087u64));
}

#[test]
fn test_generate_prime_base62_2() {
    let result = obfuskey::math::generate_prime(62, 2, 1.618033988749894848).unwrap();
    assert_eq!(result, BigUint::from(6221u64));
}

#[test]
fn test_generate_prime_base62_1() {
    let result = obfuskey::math::generate_prime(62, 1, 1.618033988749894848).unwrap();
    assert_eq!(result, BigUint::from(101u64));
}

// =============================================================================
// Multiplier auto-generation
// =============================================================================

#[test]
fn test_multiplier_auto_abc() {
    let mut obf = Obfuskey::new("abc", None, None).unwrap();
    let m = obf.multiplier().unwrap().clone();
    assert_eq!(m, BigUint::from(1181u64));
}

#[test]
fn test_set_prime_multiplier() {
    let mut obf = Obfuskey::new("abc", None, None).unwrap();
    obf.set_prime_multiplier(1.75);
    let m = obf.multiplier().unwrap().clone();
    assert_eq!(m, BigUint::from(1277u64));
}

// =============================================================================
// Obfuskey get_key — u64 API, exact cross-language vectors
// =============================================================================

#[test]
fn test_get_key_u64_base16() {
    let mut obf = Obfuskey::new(alphabets::BASE16, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "A16A63");
}

#[test]
fn test_get_key_u64_base32() {
    let mut obf = Obfuskey::new(alphabets::BASE32, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "O6VAF5");
}

#[test]
fn test_get_key_u64_base36() {
    let mut obf = Obfuskey::new(alphabets::BASE36, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "MNYJ53");
}

#[test]
fn test_get_key_u64_base52() {
    let mut obf = Obfuskey::new(alphabets::BASE52, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "ckPl95");
}

#[test]
fn test_get_key_u64_base56() {
    let mut obf = Obfuskey::new(alphabets::BASE56, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "dGTZmF");
}

#[test]
fn test_get_key_u64_base58() {
    let mut obf = Obfuskey::new(alphabets::BASE58, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "dWxtix");
}

#[test]
fn test_get_key_u64_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "d2Aasl");
}

#[test]
fn test_get_key_u64_base64() {
    let mut obf = Obfuskey::new(alphabets::BASE64, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "eIq9Uz");
}

#[test]
fn test_get_key_u64_base94() {
    let mut obf = Obfuskey::new(alphabets::BASE94, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(12345).unwrap(), "\\2'?@X");
}

// =============================================================================
// Obfuskey get_value — u64 API
// =============================================================================

#[test]
fn test_get_value_u64_base16() {
    let mut obf = Obfuskey::new(alphabets::BASE16, Some(6), None).unwrap();
    assert_eq!(obf.get_value_u64("A16A63").unwrap(), 12345u64);
}

#[test]
fn test_get_value_u64_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    assert_eq!(obf.get_value_u64("d2Aasl").unwrap(), 12345u64);
}

#[test]
fn test_get_value_u64_base94() {
    let mut obf = Obfuskey::new(alphabets::BASE94, Some(6), None).unwrap();
    assert_eq!(obf.get_value_u64("\\2'?@X").unwrap(), 12345u64);
}

// =============================================================================
// BigUint API — same vectors, verifying both paths produce identical output
// =============================================================================

#[test]
fn test_get_key_big_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    assert_eq!(obf.get_key_big(&BigUint::from(12345u64)).unwrap(), "d2Aasl");
}

#[test]
fn test_get_value_big_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    assert_eq!(obf.get_value_big("d2Aasl").unwrap(), BigUint::from(12345u64));
}

// =============================================================================
// Zero value
// =============================================================================

#[test]
fn test_get_key_zero_u64() {
    let mut obf = Obfuskey::new("abc", Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(0).unwrap(), "aaaaaa");
}

#[test]
fn test_get_value_zero_u64() {
    let mut obf = Obfuskey::new("abc", Some(6), None).unwrap();
    assert_eq!(obf.get_value_u64("aaaaaa").unwrap(), 0u64);
}

#[test]
fn test_get_key_zero_base62_u64() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    assert_eq!(obf.get_key_u64(0).unwrap(), "000000");
}

// =============================================================================
// Roundtrip tests — u64 API
// =============================================================================

#[test]
fn test_roundtrip_u64_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    for value in [0u64, 1, 12345, 1000000] {
        let key = obf.get_key_u64(value).unwrap();
        assert_eq!(key.len(), 6);
        let result = obf.get_value_u64(&key).unwrap();
        assert_eq!(result, value, "Roundtrip failed for value {}", value);
    }
}

#[test]
fn test_roundtrip_u64_base36() {
    let mut obf = Obfuskey::new(alphabets::BASE36, Some(6), None).unwrap();
    for value in [0u64, 1, 12345, 50000] {
        let key = obf.get_key_u64(value).unwrap();
        assert_eq!(key.len(), 6);
        assert_eq!(obf.get_value_u64(&key).unwrap(), value);
    }
}

#[test]
fn test_roundtrip_u64_base94() {
    let mut obf = Obfuskey::new(alphabets::BASE94, Some(6), None).unwrap();
    for value in [0u64, 1, 12345, 200000] {
        let key = obf.get_key_u64(value).unwrap();
        assert_eq!(key.len(), 6);
        assert_eq!(obf.get_value_u64(&key).unwrap(), value);
    }
}

#[test]
fn test_roundtrip_big_base62() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    for value in [0u64, 1, 12345, 1000000] {
        let v = BigUint::from(value);
        let key = obf.get_key_big(&v).unwrap();
        assert_eq!(key.len(), 6);
        assert_eq!(obf.get_value_big(&key).unwrap(), v);
    }
}

// =============================================================================
// Cross-API consistency: u64 and BigUint produce same keys
// =============================================================================

#[test]
fn test_u64_big_consistency() {
    let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), None).unwrap();
    for value in [0u64, 1, 42, 12345, 999999, 1000000] {
        let key_u64 = obf.get_key_u64(value).unwrap();
        let key_big = obf.get_key_big(&BigUint::from(value)).unwrap();
        assert_eq!(key_u64, key_big, "Mismatch for value {}", value);

        let val_u64 = obf.get_value_u64(&key_u64).unwrap();
        let val_big: u64 = obf.get_value_big(&key_big).unwrap().try_into().unwrap();
        assert_eq!(val_u64, val_big);
        assert_eq!(val_u64, value);
    }
}

// =============================================================================
// Obfusbit packing tests — u64 API
// =============================================================================

#[test]
fn test_obfusbit_pack_u64() {
    use obfuskey::{FieldSchema, Obfusbit};
    use std::collections::HashMap;

    let schema = vec![
        FieldSchema { name: "id".to_string(), bits: 10 },
        FieldSchema { name: "type".to_string(), bits: 2 },
        FieldSchema { name: "flag".to_string(), bits: 1 },
    ];

    let mut obfusbit = Obfusbit::new(schema, None).unwrap();

    let mut values = HashMap::new();
    values.insert("id".to_string(), 100u64);
    values.insert("type".to_string(), 2u64);
    values.insert("flag".to_string(), 1u64);

    match obfusbit.pack_u64(&values, false).unwrap() {
        obfuskey::PackedU64::Int(v) => assert_eq!(v, 805u64),
        _ => panic!("Expected Int"),
    }
}

#[test]
fn test_obfusbit_unpack_u64() {
    use obfuskey::{FieldSchema, Obfusbit, UnpackDataU64};

    let schema = vec![
        FieldSchema { name: "id".to_string(), bits: 10 },
        FieldSchema { name: "type".to_string(), bits: 2 },
        FieldSchema { name: "flag".to_string(), bits: 1 },
    ];

    let mut obfusbit = Obfusbit::new(schema, None).unwrap();

    let result = obfusbit
        .unpack_u64(UnpackDataU64::Int(805), false)
        .unwrap();

    assert_eq!(result["id"], 100u64);
    assert_eq!(result["type"], 2u64);
    assert_eq!(result["flag"], 1u64);
}

#[test]
fn test_obfusbit_pack_bytes_u64_big_endian() {
    use obfuskey::{ByteOrder, FieldSchema, Obfusbit};
    use std::collections::HashMap;

    let schema = vec![
        FieldSchema { name: "id".to_string(), bits: 10 },
        FieldSchema { name: "type".to_string(), bits: 2 },
        FieldSchema { name: "flag".to_string(), bits: 1 },
    ];

    let obfusbit = Obfusbit::new(schema, None).unwrap();

    let mut values = HashMap::new();
    values.insert("id".to_string(), 100u64);
    values.insert("type".to_string(), 2u64);
    values.insert("flag".to_string(), 1u64);

    let bytes = obfusbit.pack_bytes_u64(&values, ByteOrder::Big).unwrap();
    assert_eq!(bytes, vec![0x03, 0x25]);
}

#[test]
fn test_obfusbit_pack_bytes_u64_little_endian() {
    use obfuskey::{ByteOrder, FieldSchema, Obfusbit};
    use std::collections::HashMap;

    let schema = vec![
        FieldSchema { name: "id".to_string(), bits: 10 },
        FieldSchema { name: "type".to_string(), bits: 2 },
        FieldSchema { name: "flag".to_string(), bits: 1 },
    ];

    let obfusbit = Obfusbit::new(schema, None).unwrap();

    let mut values = HashMap::new();
    values.insert("id".to_string(), 100u64);
    values.insert("type".to_string(), 2u64);
    values.insert("flag".to_string(), 1u64);

    let bytes = obfusbit.pack_bytes_u64(&values, ByteOrder::Little).unwrap();
    assert_eq!(bytes, vec![0x25, 0x03]);
}

#[test]
fn test_obfusbit_roundtrip_u64_with_obfuscation() {
    use obfuskey::{FieldSchema, Obfusbit, UnpackDataU64};
    use std::collections::HashMap;

    let schema = vec![
        FieldSchema { name: "id".to_string(), bits: 10 },
        FieldSchema { name: "type".to_string(), bits: 2 },
        FieldSchema { name: "flag".to_string(), bits: 1 },
    ];

    let obfuskey = Obfuskey::new(alphabets::BASE62, Some(3), None).unwrap();
    let mut obfusbit = Obfusbit::new(schema, Some(obfuskey)).unwrap();

    let mut values = HashMap::new();
    values.insert("id".to_string(), 100u64);
    values.insert("type".to_string(), 2u64);
    values.insert("flag".to_string(), 1u64);

    let packed = obfusbit.pack_u64(&values, true).unwrap();
    let key = match packed {
        obfuskey::PackedU64::Key(k) => k,
        _ => panic!("Expected Key"),
    };

    assert_eq!(key.len(), 3);

    let result = obfusbit
        .unpack_u64(UnpackDataU64::Key(&key), true)
        .unwrap();

    assert_eq!(result["id"], 100u64);
    assert_eq!(result["type"], 2u64);
    assert_eq!(result["flag"], 1u64);
}

// =============================================================================
// Error handling tests
// =============================================================================

#[test]
fn test_duplicate_alphabet() {
    let result = Obfuskey::new("aabcdef", None, None);
    assert!(matches!(result, Err(ObfuskeyError::DuplicateError)));
}

#[test]
fn test_even_multiplier() {
    let result = Obfuskey::new("abcdef", None, Some(200));
    assert!(matches!(result, Err(ObfuskeyError::MultiplierError)));
}

#[test]
fn test_maximum_value() {
    let mut obf = Obfuskey::new("abc", Some(6), None).unwrap();
    // max = 3^6 - 1 = 728
    assert_eq!(*obf.maximum_value(), BigUint::from(728u64));

    let result = obf.get_key_u64(729);
    assert!(result.is_err());
}

#[test]
fn test_unknown_key() {
    let mut obf = Obfuskey::new("abc", Some(6), None).unwrap();
    let result = obf.get_value_u64("abcxyz");
    assert!(matches!(result, Err(ObfuskeyError::UnknownKeyError)));
}

#[test]
fn test_key_length_error() {
    let mut obf = Obfuskey::new("abc", Some(6), None).unwrap();
    let result = obf.get_value_u64("aaaaaaa"); // 7 chars, expects 6
    assert!(matches!(result, Err(ObfuskeyError::KeyLengthError)));
}

// =============================================================================
// Explicit multiplier — u64 constructor
// =============================================================================

#[test]
fn test_explicit_odd_multiplier_u64() {
    let mut obf = Obfuskey::new("abc", None, Some(127)).unwrap();
    assert_eq!(*obf.multiplier().unwrap(), BigUint::from(127u64));

    // Roundtrip
    let key = obf.get_key_u64(42).unwrap();
    assert_eq!(obf.get_value_u64(&key).unwrap(), 42);
}

// =============================================================================
// BigUint multiplier constructor
// =============================================================================

#[test]
fn test_with_big_multiplier() {
    let mut obf =
        Obfuskey::with_big_multiplier(alphabets::BASE62, 6, BigUint::from(123u64)).unwrap();

    let key = obf.get_key_big(&BigUint::from(42u64)).unwrap();
    assert_eq!(obf.get_value_big(&key).unwrap(), BigUint::from(42u64));

    let key_u64 = obf.get_key_u64(42).unwrap();
    assert_eq!(key, key_u64); // Both paths produce same result
}
