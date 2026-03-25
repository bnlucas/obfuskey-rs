use crate::error::ObfuskeyError;

/// Pre-computed alphabet with O(1) character lookups.
///
/// Stores the alphabet characters and a reverse lookup table indexed by byte value.
/// Only supports single-byte (ASCII) characters for the fast path; multi-byte
/// characters fall back to linear search.
#[derive(Clone)]
pub struct Alphabet {
    chars: Vec<char>,
    /// Reverse lookup: byte value -> index in alphabet. 0xFF = not in alphabet.
    lookup: [u8; 256],
    /// Whether all characters are single-byte ASCII (enables fast path).
    all_ascii: bool,
}

impl Alphabet {
    pub fn new(alphabet: &str) -> Result<Self, ObfuskeyError> {
        let chars: Vec<char> = alphabet.chars().collect();

        if chars.len() < 2 {
            return Err(ObfuskeyError::ValueError(
                "Alphabet must contain at least 2 characters.".to_string(),
            ));
        }

        // Check for duplicates
        let mut seen = [false; 256];
        let mut seen_set = std::collections::HashSet::new();
        let all_ascii = chars.iter().all(|c| c.is_ascii());

        for &c in &chars {
            if all_ascii {
                let b = c as u8;
                if seen[b as usize] {
                    return Err(ObfuskeyError::DuplicateError);
                }
                seen[b as usize] = true;
            } else if !seen_set.insert(c) {
                return Err(ObfuskeyError::DuplicateError);
            }
        }

        // Build reverse lookup
        let mut lookup = [0xFFu8; 256];
        if all_ascii && chars.len() <= 256 {
            for (i, &c) in chars.iter().enumerate() {
                lookup[c as usize] = i as u8;
            }
        }

        Ok(Alphabet {
            chars,
            lookup,
            all_ascii,
        })
    }

    #[inline]
    pub fn base(&self) -> usize {
        self.chars.len()
    }

    #[inline]
    pub fn char_at(&self, index: usize) -> char {
        self.chars[index]
    }

    #[inline]
    pub fn first_char(&self) -> char {
        self.chars[0]
    }

    /// Returns the index of a character, or None if not found.
    #[inline]
    pub fn index_of(&self, c: char) -> Option<usize> {
        if self.all_ascii && c.is_ascii() {
            let idx = self.lookup[c as usize];
            if idx == 0xFF {
                None
            } else {
                Some(idx as usize)
            }
        } else {
            self.chars.iter().position(|&x| x == c)
        }
    }

    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Returns the raw alphabet string.
    pub fn as_str(&self) -> String {
        self.chars.iter().collect()
    }
}

impl std::fmt::Debug for Alphabet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.as_str();
        if s.len() > 20 {
            write!(f, "Alphabet('{}...{}')", &s[..10], &s[s.len() - 5..])
        } else {
            write!(f, "Alphabet('{}')", s)
        }
    }
}
