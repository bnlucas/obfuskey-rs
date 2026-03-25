use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::error::ObfuskeyError;

// =============================================================================
// Native u64 primality (used for numbers < 2M, avoids BigUint entirely)
// =============================================================================

/// Trial division using native u64.
fn trial_division_u64(n: u64) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    let sqrt_n = (n as f64).sqrt() as u64;
    let mut i = 3;
    while i <= sqrt_n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

/// GCD using native u64.
#[inline]
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// =============================================================================
// u128 modular arithmetic (for Miller-Rabin on medium numbers)
// =============================================================================

/// Modular exponentiation: (base^exp) % modulus, using u128 to avoid overflow.
fn mod_pow_u128(mut base: u128, mut exp: u128, modulus: u128) -> u128 {
    if modulus == 1 {
        return 0;
    }
    let mut result: u128 = 1;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_mod_u128(result, base, modulus);
        }
        exp >>= 1;
        base = mul_mod_u128(base, base, modulus);
    }
    result
}

/// Multiply two u128 values modulo m without overflow.
/// Uses the fact that if both a, b < m and m < 2^64, then a*b fits in u128.
/// For larger m, uses Russian peasant multiplication.
#[inline]
fn mul_mod_u128(a: u128, b: u128, m: u128) -> u128 {
    if m <= u64::MAX as u128 {
        // Both a, b < m <= 2^64, so a*b <= (2^64-1)^2 < 2^128. Safe.
        (a * b) % m
    } else {
        // Russian peasant multiplication for very large moduli
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

/// Factor n-1 = 2^s * d where d is odd. Native u128 version.
fn factor_u128(n: u128) -> (u32, u128) {
    let mut s = 0u32;
    let mut d = n - 1;
    while d & 1 == 0 {
        s += 1;
        d >>= 1;
    }
    (s, d)
}

/// Miller-Rabin strong pseudoprime test using native u128.
fn strong_pseudoprime_u128(n: u128, base: u128) -> bool {
    if n & 1 == 0 || n <= 1 {
        return false;
    }

    let (s, d) = factor_u128(n);
    let n_minus_1 = n - 1;
    let mut x = mod_pow_u128(base, d, n);

    if x == 1 || x == n_minus_1 {
        return true;
    }

    for _ in 0..s - 1 {
        x = mul_mod_u128(x, x, n);
        if x == n_minus_1 {
            return true;
        }
        if x == 1 {
            return false;
        }
    }

    false
}

/// Deterministic Miller-Rabin for numbers that fit in u128.
/// Bases [2, 13, 23, 1662803] are sufficient for n < 2,047,698,921.
fn small_strong_pseudoprime_u128(n: u128) -> bool {
    for base in [2u128, 13, 23, 1662803] {
        if !strong_pseudoprime_u128(n, base) {
            return false;
        }
    }
    true
}

// =============================================================================
// BigUint primality (fallback for numbers > u128::MAX)
// =============================================================================

fn factor_big(n: &BigUint) -> (u64, BigUint) {
    let mut s: u64 = 0;
    let mut d = n - BigUint::one();
    let two = BigUint::from(2u32);

    while (&d % &two).is_zero() {
        s += 1;
        d /= &two;
    }

    (s, d)
}

fn strong_pseudoprime_big(n: &BigUint, base: u64) -> bool {
    let (s, d) = factor_big(n);
    let n_minus_1 = n - BigUint::one();
    let mut x = BigUint::from(base).modpow(&d, n);

    if x == BigUint::one() || x == n_minus_1 {
        return true;
    }

    let two = BigUint::from(2u32);
    for _ in 0..s - 1 {
        x = x.modpow(&two, n);
        if x == n_minus_1 {
            return true;
        }
        if x == BigUint::one() {
            return false;
        }
    }

    false
}

// =============================================================================
// Public primality API
// =============================================================================

/// Determines if an integer is prime.
pub fn is_prime(n: &BigUint) -> bool {
    // Fast path: if it fits in u64, use native arithmetic
    if let Ok(n_u64) = TryInto::<u64>::try_into(n) {
        return is_prime_u64(n_u64);
    }

    // For large numbers, use BigUint Miller-Rabin
    if let Ok(n_u128) = TryInto::<u128>::try_into(n) {
        return is_prime_u128(n_u128);
    }

    // Very large: BigUint path
    let two = BigUint::from(2u32);
    if (n % &two).is_zero() {
        return false;
    }
    for base in [2u64, 13, 23, 1662803] {
        if !strong_pseudoprime_big(n, base) {
            return false;
        }
    }
    true
}

#[inline]
fn is_prime_u64(n: u64) -> bool {
    if n == 2 {
        return true;
    }
    if n < 2 || n % 2 == 0 {
        return false;
    }

    // gcd(n, 510510) > 1 check — 510510 = 2*3*5*7*11*13*17
    if gcd_u64(n, 510510) > 1 {
        return matches!(n, 3 | 5 | 7 | 11 | 13 | 17);
    }

    if n < 2_000_000 {
        return trial_division_u64(n);
    }

    small_strong_pseudoprime_u128(n as u128)
}

fn is_prime_u128(n: u128) -> bool {
    if n == 2 {
        return true;
    }
    if n < 2 || n % 2 == 0 {
        return false;
    }

    // gcd check (small factor sieve)
    if n <= u64::MAX as u128 {
        return is_prime_u64(n as u64);
    }

    // Large u128: Miller-Rabin
    small_strong_pseudoprime_u128(n)
}

// =============================================================================
// Next prime
// =============================================================================

const GAP: [u64; 30] = [
    1, 6, 5, 4, 3, 2, 1, 4, 3, 2, 1, 2, 1, 4, 3, 2, 1, 2, 1, 4, 3, 2, 1, 6, 5, 4, 3, 2, 1, 2,
];

/// Find the next prime strictly greater than n, using native u128 arithmetic.
fn next_prime_u128(n: u128) -> u128 {
    if n < 2 {
        return 2;
    }
    if n < 5 {
        return match n {
            2 => 3,
            3 | 4 => 5,
            _ => unreachable!(),
        };
    }

    let mut candidate = n + 1 + (n & 1);

    if candidate % 3 == 0 || candidate % 5 == 0 {
        candidate += GAP[(candidate % 30) as usize] as u128;
    }

    while !is_prime_u128(candidate) {
        candidate += GAP[(candidate % 30) as usize] as u128;
    }

    candidate
}

/// Finds the next prime number strictly greater than n.
pub fn next_prime(n: &BigUint) -> Result<BigUint, ObfuskeyError> {
    if n.bits() > 512 {
        return Err(ObfuskeyError::MaximumValueError(
            "For integers larger than 512-bit, prime generation is not supported.".to_string(),
        ));
    }

    // Fast path: if it fits in u128
    if let Ok(n_u128) = TryInto::<u128>::try_into(n) {
        return Ok(BigUint::from(next_prime_u128(n_u128)));
    }

    // BigUint fallback for very large numbers
    let two = BigUint::from(2u32);
    if *n < two {
        return Ok(two);
    }
    if *n < BigUint::from(5u32) {
        let result = match TryInto::<u64>::try_into(n).unwrap() {
            2 => 3u64,
            3 | 4 => 5,
            _ => unreachable!(),
        };
        return Ok(BigUint::from(result));
    }

    let mut candidate = n.clone() + BigUint::one() + (n & BigUint::one());

    let thirty = BigUint::from(30u32);
    let three = BigUint::from(3u32);
    let five = BigUint::from(5u32);

    if (&candidate % &three).is_zero() || (&candidate % &five).is_zero() {
        let idx: usize = (&candidate % &thirty).try_into().unwrap();
        candidate += BigUint::from(GAP[idx]);
    }

    while !is_prime(&candidate) {
        let idx: usize = (&candidate % &thirty).try_into().unwrap();
        candidate += BigUint::from(GAP[idx]);
    }

    Ok(candidate)
}

// =============================================================================
// Modular inverse
// =============================================================================

/// Modular inverse using native u128. Returns x such that (base * x) % modulus == 1.
pub fn mod_inv_u128(base: u128, modulus: u128) -> Result<u128, ObfuskeyError> {
    let base = (base % modulus) as i128;
    let modulus_i = modulus as i128;

    let mut r_prev = modulus_i;
    let mut r_curr = base;
    let mut t_prev: i128 = 0;
    let mut t_curr: i128 = 1;

    while r_curr != 0 {
        let q = r_prev / r_curr;

        let temp_r = r_prev - q * r_curr;
        r_prev = r_curr;
        r_curr = temp_r;

        let temp_t = t_prev - q * t_curr;
        t_prev = t_curr;
        t_curr = temp_t;
    }

    if r_prev != 1 {
        return Err(ObfuskeyError::ValueError(
            "Modular inverse does not exist (values are not coprime).".to_string(),
        ));
    }

    Ok(((t_prev % modulus_i + modulus_i) % modulus_i) as u128)
}

/// Modular inverse using BigUint. Returns x such that (base * x) % modulus == 1.
pub fn mod_inv_big(base: &BigUint, modulus: &BigUint) -> Result<BigUint, ObfuskeyError> {
    use num_bigint::BigInt;

    let mod_int = BigInt::from(modulus.clone());
    let base_int = BigInt::from(base % modulus);

    let mut r_prev = mod_int.clone();
    let mut r_curr = base_int;
    let mut t_prev = BigInt::zero();
    let mut t_curr = BigInt::one();

    while !r_curr.is_zero() {
        let q = &r_prev / &r_curr;

        let temp_r = &r_prev - &q * &r_curr;
        r_prev = r_curr;
        r_curr = temp_r;

        let temp_t = &t_prev - &q * &t_curr;
        t_prev = t_curr;
        t_curr = temp_t;
    }

    if r_prev != BigInt::one() {
        return Err(ObfuskeyError::ValueError(
            "Modular inverse does not exist (values are not coprime).".to_string(),
        ));
    }

    let result = ((t_prev % &mod_int) + &mod_int) % &mod_int;
    Ok(result.to_biguint().unwrap())
}

// =============================================================================
// Prime generation
// =============================================================================

/// Generates a prime multiplier for obfuscation.
/// Uses fixed-point arithmetic (scale = 10^18) to match Python/JS precision.
pub fn generate_prime(
    base: usize,
    key_length: u32,
    prime_multiplier: f64,
) -> Result<BigUint, ObfuskeyError> {
    let base_big = BigUint::from(base);
    let max_value = base_big.pow(key_length) - BigUint::one();

    let scale: u64 = 1_000_000_000_000_000_000; // 10^18
    let multiplier_scaled = (prime_multiplier * scale as f64) as u64;

    let target = (&max_value * BigUint::from(multiplier_scaled)) / BigUint::from(scale);

    next_prime(&target)
}
