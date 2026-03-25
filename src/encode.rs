use num_bigint::BigUint;
use num_traits::Zero;

use crate::alphabet::Alphabet;

/// Encode a u128 value to a string using the given alphabet.
/// Uses only stack allocation for the result buffer.
pub fn encode_u128(value: u128, alphabet: &Alphabet) -> String {
    let base = alphabet.base() as u128;

    if value < base {
        return alphabet.char_at(value as usize).to_string();
    }

    // Max digits for u128 in base 2 is 128; base 94 needs ~20 digits
    let mut buf = [0u8; 128];
    let mut pos = 128;
    let mut val = value;

    while val > 0 {
        pos -= 1;
        buf[pos] = (val % base) as u8;
        val /= base;
    }

    let mut result = String::with_capacity(128 - pos);
    for &idx in &buf[pos..] {
        result.push(alphabet.char_at(idx as usize));
    }
    result
}

/// Encode a BigUint value to a string using the given alphabet.
pub fn encode_big(value: &BigUint, alphabet: &Alphabet) -> String {
    let base = BigUint::from(alphabet.base());

    if *value < base {
        let idx: usize = value.try_into().unwrap();
        return alphabet.char_at(idx).to_string();
    }

    let mut key = Vec::new();
    let mut val = value.clone();

    while !val.is_zero() {
        let remainder = &val % &base;
        val /= &base;
        let idx: usize = remainder.try_into().unwrap();
        key.push(alphabet.char_at(idx));
    }

    key.reverse();
    key.into_iter().collect()
}
