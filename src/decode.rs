use num_bigint::BigUint;

use crate::alphabet::Alphabet;
use crate::error::ObfuskeyError;

/// Decode a string to u128 using the given alphabet.
/// Returns None if the result would overflow u128.
pub fn decode_u128(value: &str, alphabet: &Alphabet) -> Result<Option<u128>, ObfuskeyError> {
    let base = alphabet.base() as u128;

    // Horner's method: result = result * base + digit (left to right, no reversal needed)
    let mut result: u128 = 0;

    for c in value.chars() {
        let idx = alphabet
            .index_of(c)
            .ok_or(ObfuskeyError::UnknownKeyError)? as u128;

        // Check for overflow before multiply
        result = match result.checked_mul(base) {
            Some(v) => match v.checked_add(idx) {
                Some(v) => v,
                None => return Ok(None),
            },
            None => return Ok(None),
        };
    }

    Ok(Some(result))
}

/// Decode a string to BigUint using the given alphabet.
/// Uses Horner's method for efficiency (no base^i computation).
pub fn decode_big(value: &str, alphabet: &Alphabet) -> Result<BigUint, ObfuskeyError> {
    let base = BigUint::from(alphabet.base());

    // Horner's method: left to right
    let mut result = BigUint::ZERO;

    for c in value.chars() {
        let idx = alphabet
            .index_of(c)
            .ok_or(ObfuskeyError::UnknownKeyError)?;

        result = result * &base + BigUint::from(idx);
    }

    Ok(result)
}
