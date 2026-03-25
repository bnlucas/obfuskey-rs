# Obfuskey: Integer Packing and Obfuscation Library

**Note:** This library is a Rust port of the [Obfuskey Python package](https://github.com/bnlucas/obfuskey) and is fully cross-language compatible with both the Python and [TypeScript/JavaScript](https://github.com/bnlucas/obfuskey-js) implementations. Keys generated in any language can be decoded in any other.

Obfuskey is a Rust library for efficient packing and unpacking of multiple integer values into a single large integer, and for obfuscation/de-obfuscation of these integers into short, human-readable string "keys". It's ideal for scenarios requiring compact data representation (e.g., URL parameters, short identifiers) with optional obfuscation.

The combination of key length and alphabet used will determine the maximum value it can obfuscate: `alphabet.len() ^ key_length - 1`.

## Features

* **Bit-packing**: Define a schema to pack multiple integer fields into a single integer or byte array.
* **Bit-unpacking**: Retrieve individual integer fields from a packed integer or byte array.
* **Dual-path arithmetic**: Optimized `u128` fast path for common cases with automatic `BigUint` fallback for large key spaces.
* **Native integer API**: Ergonomic `u64` methods (`get_key_u64`, `get_value_u64`) alongside `BigUint` methods for large values.
* **Customizable alphabets**: Define your own character sets for key generation, supporting base conversion up to Base94+.
* **O(1) alphabet lookups**: Pre-computed lookup table for ASCII alphabets.
* **Obfuscation**: Scramble integers into short, unique string keys using a configurable prime multiplier.
* **Cross-language compatible**: Produces identical output to the Python and TypeScript implementations.
* **Error handling**: Comprehensive error types for clear debugging.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
obfuskey = "0.1"
```

## Usage

### Obfuskey - Basic Usage

```rust
use obfuskey::{alphabets, Obfuskey};

// Using default multiplier (auto-generated prime)
let mut obfuscator = Obfuskey::new(alphabets::BASE62, Some(8), None).unwrap();

let key = obfuscator.get_key_u64(1234567890).unwrap();
let value = obfuscator.get_value_u64(&key).unwrap();
assert_eq!(value, 1234567890);

// Using a specific multiplier (must be odd)
let mut obf = Obfuskey::new(alphabets::BASE62, Some(6), Some(46485)).unwrap();
let key = obf.get_key_u64(12345).unwrap();
println!("Key: {}", key);

// Using a custom alphabet
let mut custom = Obfuskey::new("012345abcdef", Some(6), None).unwrap();
let key = custom.get_key_u64(123).unwrap();
println!("Custom alphabet key: {}", key);
```

### BigUint API for Large Values

For key spaces that exceed `u64` range, use the `BigUint` API:

```rust
use num_bigint::BigUint;
use obfuskey::{alphabets, Obfuskey};

let mut obf = Obfuskey::new(alphabets::BASE62, Some(20), None).unwrap();

let large_value = BigUint::from(u128::MAX);
let key = obf.get_key_big(&large_value).unwrap();
let result = obf.get_value_big(&key).unwrap();
assert_eq!(result, large_value);
```

### Obfusbit - Packing Multiple Values

Obfusbit allows you to pack multiple integer values into a single obfuscated key or raw integer.

#### Basic Packing and Unpacking

```rust
use obfuskey::{FieldSchema, Obfusbit, PackedU64, UnpackDataU64};
use std::collections::HashMap;

let schema = vec![
    FieldSchema { name: "category_id".to_string(), bits: 4 },  // Max 15
    FieldSchema { name: "item_id".to_string(), bits: 20 },     // Max ~1M
    FieldSchema { name: "status".to_string(), bits: 3 },       // Max 7
];

let mut packer = Obfusbit::new(schema, None).unwrap();

let mut values = HashMap::new();
values.insert("category_id".to_string(), 5u64);
values.insert("item_id".to_string(), 123456u64);
values.insert("status".to_string(), 1u64);

// Pack into a u64
let packed = packer.pack_u64(&values, false).unwrap();
if let PackedU64::Int(v) = packed {
    println!("Packed: {}", v);
}

// Unpack back
let unpacked = packer.unpack_u64(UnpackDataU64::Int(805), false).unwrap();
assert_eq!(unpacked["category_id"], 5);
```

#### Packing with Obfuscation

```rust
use obfuskey::{alphabets, FieldSchema, Obfusbit, Obfuskey, PackedU64, UnpackDataU64};
use std::collections::HashMap;

let schema = vec![
    FieldSchema { name: "id".to_string(), bits: 10 },
    FieldSchema { name: "type".to_string(), bits: 2 },
    FieldSchema { name: "flag".to_string(), bits: 1 },
];

let obfuskey = Obfuskey::new(alphabets::BASE62, Some(3), None).unwrap();
let mut packer = Obfusbit::new(schema, Some(obfuskey)).unwrap();

let mut values = HashMap::new();
values.insert("id".to_string(), 100u64);
values.insert("type".to_string(), 2u64);
values.insert("flag".to_string(), 1u64);

// Pack and obfuscate to a string key
let packed = packer.pack_u64(&values, true).unwrap();
let key = match packed {
    PackedU64::Key(k) => k,
    _ => unreachable!(),
};
println!("Obfuscated key: {}", key); // 3-character string

// Unpack and de-obfuscate
let result = packer.unpack_u64(UnpackDataU64::Key(&key), true).unwrap();
assert_eq!(result["id"], 100);
assert_eq!(result["type"], 2);
assert_eq!(result["flag"], 1);
```

#### Byte Serialization

```rust
use obfuskey::{ByteOrder, FieldSchema, Obfusbit};
use std::collections::HashMap;

let schema = vec![
    FieldSchema { name: "id".to_string(), bits: 10 },
    FieldSchema { name: "type".to_string(), bits: 2 },
    FieldSchema { name: "flag".to_string(), bits: 1 },
];

let packer = Obfusbit::new(schema, None).unwrap();

let mut values = HashMap::new();
values.insert("id".to_string(), 100u64);
values.insert("type".to_string(), 2u64);
values.insert("flag".to_string(), 1u64);

let bytes = packer.pack_bytes_u64(&values, ByteOrder::Big).unwrap();
assert_eq!(bytes, vec![0x03, 0x25]);

let unpacked = packer.unpack_bytes_u64(&bytes, ByteOrder::Big).unwrap();
assert_eq!(unpacked["id"], 100);
```

### Determining Key Length for Obfusbit

When using `Obfusbit` with `Obfuskey`, the obfuscator must handle the maximum value your schema can produce:

1. **Total bits** = sum of all field bits in the schema
2. **Max schema value** = `2^total_bits - 1`
3. **Max obfuskey value** = `alphabet_size^key_length - 1`

The obfuskey max must be >= the schema max. To find the minimum `key_length`:

```rust
let total_bits: u32 = 27; // sum of your schema bits
let alphabet_size: f64 = 58.0; // BASE58
let min_key_length = (total_bits as f64 / alphabet_size.log2()).ceil() as u32;
println!("Minimum key_length: {}", min_key_length); // 5
```

### Large Integer Support

This library uses `num-bigint` for arbitrary-precision arithmetic. The prime generation function supports integers up to 512 bits. For most practical use cases (key lengths up to ~21 with BASE62), the optimized `u128` fast path is used automatically.

## API Reference

### Structs

#### `Obfuskey`

```rust
impl Obfuskey {
    // Construction
    fn new(alphabet: &str, key_length: Option<u32>, multiplier: Option<u64>) -> Result<Self, ObfuskeyError>;
    fn with_big_multiplier(alphabet: &str, key_length: u32, multiplier: BigUint) -> Result<Self, ObfuskeyError>;

    // u64 API (fast path)
    fn get_key_u64(&mut self, value: u64) -> Result<String, ObfuskeyError>;
    fn get_value_u64(&mut self, key: &str) -> Result<u64, ObfuskeyError>;

    // BigUint API (large values)
    fn get_key_big(&mut self, value: &BigUint) -> Result<String, ObfuskeyError>;
    fn get_value_big(&mut self, key: &str) -> Result<BigUint, ObfuskeyError>;

    // Properties
    fn alphabet(&self) -> &Alphabet;
    fn key_length(&self) -> u32;
    fn maximum_value(&self) -> &BigUint;
    fn multiplier(&mut self) -> Result<&BigUint, ObfuskeyError>;
    fn set_prime_multiplier(&mut self, multiplier: f64);
}
```

#### `Obfusbit`

```rust
impl Obfusbit {
    fn new(schema: Vec<FieldSchema>, obfuskey: Option<Obfuskey>) -> Result<Self, ObfuskeyError>;
    fn from_schema(schema: ObfusbitSchema, obfuskey: Option<Obfuskey>) -> Result<Self, ObfuskeyError>;

    // u64 API
    fn pack_u64(&mut self, values: &HashMap<String, u64>, obfuscate: bool) -> Result<PackedU64, ObfuskeyError>;
    fn unpack_u64(&mut self, data: UnpackDataU64, obfuscated: bool) -> Result<HashMap<String, u64>, ObfuskeyError>;
    fn pack_bytes_u64(&self, values: &HashMap<String, u64>, byteorder: ByteOrder) -> Result<Vec<u8>, ObfuskeyError>;
    fn unpack_bytes_u64(&self, data: &[u8], byteorder: ByteOrder) -> Result<HashMap<String, u64>, ObfuskeyError>;

    // BigUint API
    fn pack_big(&mut self, values: &HashMap<String, BigUint>, obfuscate: bool) -> Result<PackedBig, ObfuskeyError>;
    fn unpack_big(&mut self, data: UnpackDataBig, obfuscated: bool) -> Result<HashMap<String, BigUint>, ObfuskeyError>;
    fn pack_bytes_big(&mut self, values: &HashMap<String, BigUint>, byteorder: ByteOrder) -> Result<Vec<u8>, ObfuskeyError>;
    fn unpack_bytes_big(&self, data: &[u8], byteorder: ByteOrder) -> Result<HashMap<String, BigUint>, ObfuskeyError>;
}
```

### Pre-defined Alphabets

| Constant | Characters | Base |
|---|---|---|
| `BASE16` | `0123456789ABCDEF` | 16 |
| `BASE32` | `234567ABCDEFGHIJKLMNOPQRSTUVWXYZ` | 32 |
| `BASE36` | `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ` | 36 |
| `BASE52` | `0-9, consonants upper/lower` | 52 |
| `BASE56` | `2-9, A-Z, a-z (no ambiguous chars)` | 56 |
| `BASE58` | `1-9, A-Z, a-z (Bitcoin-style)` | 58 |
| `BASE62` | `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz` | 62 |
| `BASE64` | `BASE62 + +/` | 64 |
| `BASE64_URL_SAFE` | `BASE62 + -_` | 64 |
| `BASE94` | All printable ASCII (`!` through `~`) | 94 |
| `CROCKFORD_BASE32` | `0-9, A-Z (no I, L, O, U)` | 32 |
| `ZBASE32` | `ybndrfg8ejkmcpqxot1uwisza345h769` | 32 |

### Errors

All errors are variants of `ObfuskeyError`:

| Variant | Description |
|---|---|
| `DuplicateError` | Alphabet contains duplicate characters |
| `MultiplierError` | Multiplier is not an odd integer |
| `MaximumValueError` | Value exceeds the maximum allowed |
| `UnknownKeyError` | Key contains characters not in the alphabet |
| `KeyLengthError` | Key length doesn't match expected length |
| `SchemaValidationError` | Invalid schema definition |
| `BitOverflowError` | Field value exceeds allocated bits |
| `ValueError` | General invalid value |

## Cross-Language Compatibility

This crate produces identical output to the Python and TypeScript implementations. For example, with `BASE62` alphabet and `key_length=6`:

| Value | Key |
|---|---|
| `12345` | `d2Aasl` |
| `0` | `000000` |

A key generated in Python (`obfuskey.get_key(12345)`) can be decoded in Rust (`obf.get_value_u64("d2Aasl")`) and vice versa, as long as the same alphabet and key length are used.

## Contributing

Contributions are welcome! Please feel free to open issues or submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.
