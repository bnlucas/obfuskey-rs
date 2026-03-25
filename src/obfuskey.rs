use num_bigint::BigUint;
use num_traits::Zero;

use crate::alphabet::Alphabet;
use crate::decode::{decode_big, decode_u128};
use crate::encode::{encode_big, encode_u128};
use crate::error::ObfuskeyError;
use crate::math::{generate_prime, mod_inv_big, mod_inv_u128};

const DEFAULT_KEY_LENGTH: u32 = 6;
const PRIME_MULTIPLIER: f64 = 1.618033988749894848;

pub struct Obfuskey {
    alphabet: Alphabet,
    key_length: u32,
    maximum_value: BigUint,
    /// Cached modulus as u128 if it fits (enables fast path).
    modulus_u128: Option<u128>,
    multiplier: Option<BigUint>,
    /// Cached multiplier as u128 if it fits.
    multiplier_u128: Option<u128>,
    /// Cached modular inverse as u128 if it fits.
    inverse_u128: Option<u128>,
    prime_multiplier: f64,
}

impl Obfuskey {
    pub fn new(
        alphabet: &str,
        key_length: Option<u32>,
        multiplier: Option<u64>,
    ) -> Result<Self, ObfuskeyError> {
        let key_length = key_length.unwrap_or(DEFAULT_KEY_LENGTH);
        let alpha = Alphabet::new(alphabet)?;

        // Validate multiplier is odd if provided
        if let Some(m) = multiplier {
            if m % 2 == 0 {
                return Err(ObfuskeyError::MultiplierError);
            }
        }

        let base = BigUint::from(alpha.base());
        let maximum_value = base.pow(key_length) - BigUint::from(1u32);
        let modulus = &maximum_value + BigUint::from(1u32);

        let modulus_u128: Option<u128> = (&modulus).try_into().ok();

        let multiplier_big = multiplier.map(BigUint::from);
        let multiplier_u128 = multiplier;

        // Pre-compute inverse if we have a u128 fast path
        let inverse_u128 = match (modulus_u128, multiplier_u128) {
            (Some(m), Some(mult)) => mod_inv_u128(mult as u128, m).ok(),
            _ => None,
        };

        Ok(Obfuskey {
            alphabet: alpha,
            key_length,
            maximum_value,
            modulus_u128,
            multiplier: multiplier_big,
            multiplier_u128: multiplier.map(|m| m as u128),
            inverse_u128,
            prime_multiplier: PRIME_MULTIPLIER,
        })
    }

    /// Construct with a BigUint multiplier for very large key spaces.
    pub fn with_big_multiplier(
        alphabet: &str,
        key_length: u32,
        multiplier: BigUint,
    ) -> Result<Self, ObfuskeyError> {
        let alpha = Alphabet::new(alphabet)?;

        if (&multiplier % BigUint::from(2u32)).is_zero() {
            return Err(ObfuskeyError::MultiplierError);
        }

        let base = BigUint::from(alpha.base());
        let maximum_value = base.pow(key_length) - BigUint::from(1u32);
        let modulus = &maximum_value + BigUint::from(1u32);
        let modulus_u128: Option<u128> = (&modulus).try_into().ok();
        let multiplier_u128: Option<u128> = (&multiplier).try_into().ok();

        let inverse_u128 = match (modulus_u128, multiplier_u128) {
            (Some(m), Some(mult)) => mod_inv_u128(mult, m).ok(),
            _ => None,
        };

        Ok(Obfuskey {
            alphabet: alpha,
            key_length,
            maximum_value,
            modulus_u128,
            multiplier_u128,
            multiplier: Some(multiplier),
            inverse_u128,
            prime_multiplier: PRIME_MULTIPLIER,
        })
    }

    pub fn alphabet(&self) -> &Alphabet {
        &self.alphabet
    }

    pub fn key_length(&self) -> u32 {
        self.key_length
    }

    pub fn maximum_value(&self) -> &BigUint {
        &self.maximum_value
    }

    /// Returns the multiplier, generating it if needed.
    pub fn multiplier(&mut self) -> Result<&BigUint, ObfuskeyError> {
        if self.multiplier.is_none() {
            self.generate_multiplier()?;
        }
        Ok(self.multiplier.as_ref().unwrap())
    }

    pub fn set_prime_multiplier(&mut self, multiplier: f64) {
        self.prime_multiplier = multiplier;
        self.multiplier = None;
        self.multiplier_u128 = None;
        self.inverse_u128 = None;
    }

    // =========================================================================
    // u64 convenience API
    // =========================================================================

    /// Obfuscate a u64 value to a fixed-length key string.
    pub fn get_key_u64(&mut self, value: u64) -> Result<String, ObfuskeyError> {
        self.ensure_multiplier()?;

        if let Some(modulus) = self.modulus_u128 {
            let max_val = modulus - 1;
            if value as u128 > max_val {
                return Err(ObfuskeyError::MaximumValueError(format!(
                    "The maximum value possible is {}",
                    self.maximum_value
                )));
            }

            if value == 0 {
                return Ok(self.zero_key());
            }

            let mult = self.multiplier_u128.unwrap();
            let obfuscated = mul_mod_u128(value as u128, mult, modulus);
            let encoded = encode_u128(obfuscated, &self.alphabet);
            return Ok(self.pad_key(&encoded));
        }

        // Fallback to BigUint
        self.get_key_big(&BigUint::from(value))
    }

    /// Deobfuscate a key string back to a u64 value.
    pub fn get_value_u64(&mut self, key: &str) -> Result<u64, ObfuskeyError> {
        self.validate_key(key)?;

        if self.is_zero_key(key) {
            return Ok(0);
        }

        self.ensure_multiplier()?;

        if let (Some(modulus), Some(inverse)) = (self.modulus_u128, self.inverse_u128) {
            let decoded = decode_u128(key, &self.alphabet)?;
            if let Some(decoded_val) = decoded {
                let result = mul_mod_u128(decoded_val, inverse, modulus);
                return Ok(result as u64);
            }
        }

        // Fallback
        let result = self.get_value_big(key)?;
        Ok(result.try_into().map_err(|_| {
            ObfuskeyError::MaximumValueError("Value does not fit in u64.".to_string())
        })?)
    }

    // =========================================================================
    // BigUint API (for large key spaces)
    // =========================================================================

    /// Obfuscate a BigUint value to a fixed-length key string.
    pub fn get_key_big(&mut self, value: &BigUint) -> Result<String, ObfuskeyError> {
        if *value > self.maximum_value {
            return Err(ObfuskeyError::MaximumValueError(format!(
                "The maximum value possible is {}",
                self.maximum_value
            )));
        }

        if value.is_zero() {
            return Ok(self.zero_key());
        }

        self.ensure_multiplier()?;

        // Try u128 fast path
        if let (Some(modulus), Some(mult)) = (self.modulus_u128, self.multiplier_u128) {
            if let Ok(val_u128) = TryInto::<u128>::try_into(value) {
                let obfuscated = mul_mod_u128(val_u128, mult, modulus);
                let encoded = encode_u128(obfuscated, &self.alphabet);
                return Ok(self.pad_key(&encoded));
            }
        }

        // BigUint path
        let modulus = &self.maximum_value + BigUint::from(1u32);
        let obfuscated = (value * self.multiplier.as_ref().unwrap()) % &modulus;
        let encoded = encode_big(&obfuscated, &self.alphabet);
        Ok(self.pad_key(&encoded))
    }

    /// Deobfuscate a key string back to a BigUint value.
    pub fn get_value_big(&mut self, key: &str) -> Result<BigUint, ObfuskeyError> {
        self.validate_key(key)?;

        if self.is_zero_key(key) {
            return Ok(BigUint::zero());
        }

        self.ensure_multiplier()?;

        // Try u128 fast path
        if let (Some(modulus), Some(inverse)) = (self.modulus_u128, self.inverse_u128) {
            if let Some(decoded_val) = decode_u128(key, &self.alphabet)? {
                let result = mul_mod_u128(decoded_val, inverse, modulus);
                return Ok(BigUint::from(result));
            }
        }

        // BigUint path
        let decoded = decode_big(key, &self.alphabet)?;
        let modulus = &self.maximum_value + BigUint::from(1u32);
        let inverse = mod_inv_big(self.multiplier.as_ref().unwrap(), &modulus)?;
        Ok((decoded * inverse) % modulus)
    }

    // =========================================================================
    // Internals
    // =========================================================================

    fn ensure_multiplier(&mut self) -> Result<(), ObfuskeyError> {
        if self.multiplier.is_none() {
            self.generate_multiplier()?;
        }
        Ok(())
    }

    fn generate_multiplier(&mut self) -> Result<(), ObfuskeyError> {
        let prime = generate_prime(
            self.alphabet.base(),
            self.key_length,
            self.prime_multiplier,
        )?;

        self.multiplier_u128 = (&prime).try_into().ok();

        // Pre-compute inverse for fast path
        if let (Some(modulus), Some(mult)) = (self.modulus_u128, self.multiplier_u128) {
            self.inverse_u128 = mod_inv_u128(mult, modulus).ok();
        }

        self.multiplier = Some(prime);
        Ok(())
    }

    fn validate_key(&self, key: &str) -> Result<(), ObfuskeyError> {
        // Validate characters
        for c in key.chars() {
            if self.alphabet.index_of(c).is_none() {
                return Err(ObfuskeyError::UnknownKeyError);
            }
        }

        if key.chars().count() != self.key_length as usize {
            return Err(ObfuskeyError::KeyLengthError);
        }

        Ok(())
    }

    #[inline]
    fn zero_key(&self) -> String {
        std::iter::repeat(self.alphabet.first_char())
            .take(self.key_length as usize)
            .collect()
    }

    #[inline]
    fn is_zero_key(&self, key: &str) -> bool {
        let first = self.alphabet.first_char();
        key.chars().all(|c| c == first)
    }

    #[inline]
    fn pad_key(&self, encoded: &str) -> String {
        let len = self.key_length as usize;
        let encoded_len = encoded.len();
        if encoded_len >= len {
            return encoded.to_string();
        }
        let pad_char = self.alphabet.first_char();
        let mut result = String::with_capacity(len);
        for _ in 0..len - encoded_len {
            result.push(pad_char);
        }
        result.push_str(encoded);
        result
    }
}

/// Multiply two u128 values modulo m without overflow.
#[inline]
fn mul_mod_u128(a: u128, b: u128, m: u128) -> u128 {
    if m <= u64::MAX as u128 {
        // Both operands < m <= 2^64, product fits in u128
        (a * b) % m
    } else {
        // Russian peasant multiplication
        let mut result: u128 = 0;
        let mut a = a % m;
        let mut b = b;
        while b > 0 {
            if b & 1 == 1 {
                result = (result + a) % m;
            }
            a = (a + a) % m;
            b >>= 1;
        }
        result
    }
}

impl std::fmt::Display for Obfuskey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.alphabet.as_str();
        let alpha_display = if s.len() > 20 {
            format!("'{}...{}'", &s[..10], &s[s.len() - 5..])
        } else {
            format!("'{}'", s)
        };

        let multiplier_info = match &self.multiplier {
            Some(m) => format!(", multiplier={}", m),
            None => format!(", multiplier=auto (prime_mult={})", self.prime_multiplier),
        };

        write!(
            f,
            "Obfuskey(alphabet={}, key_length={}{})",
            alpha_display, self.key_length, multiplier_info
        )
    }
}

impl std::fmt::Debug for Obfuskey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
