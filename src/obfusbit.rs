use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::collections::HashMap;

use crate::error::ObfuskeyError;
use crate::obfusbit_schema::{FieldSchema, ObfusbitSchema};
use crate::obfuskey::Obfuskey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Big,
    Little,
}

pub struct Obfusbit {
    schema: ObfusbitSchema,
    obfuskey: Option<Obfuskey>,
}

impl Obfusbit {
    pub fn new(
        schema: Vec<FieldSchema>,
        obfuskey: Option<Obfuskey>,
    ) -> Result<Self, ObfuskeyError> {
        let schema = ObfusbitSchema::new(schema)?;

        if let Some(ref ok) = obfuskey {
            if schema.maximum_value() > ok.maximum_value() {
                return Err(ObfuskeyError::MaximumValueError(format!(
                    "Schema maximum value {} exceeds obfuskey maximum value {}.",
                    schema.maximum_value(),
                    ok.maximum_value()
                )));
            }
        }

        Ok(Obfusbit { schema, obfuskey })
    }

    pub fn from_schema(
        schema: ObfusbitSchema,
        obfuskey: Option<Obfuskey>,
    ) -> Result<Self, ObfuskeyError> {
        if let Some(ref ok) = obfuskey {
            if schema.maximum_value() > ok.maximum_value() {
                return Err(ObfuskeyError::MaximumValueError(format!(
                    "Schema maximum value {} exceeds obfuskey maximum value {}.",
                    schema.maximum_value(),
                    ok.maximum_value()
                )));
            }
        }

        Ok(Obfusbit { schema, obfuskey })
    }

    pub fn schema(&self) -> &ObfusbitSchema {
        &self.schema
    }

    pub fn total_bits(&self) -> u32 {
        self.schema.total_bits()
    }

    pub fn maximum_value(&self) -> &BigUint {
        self.schema.maximum_value()
    }

    // =========================================================================
    // u64 API — fast path using native integers
    // =========================================================================

    /// Pack u64 field values into a single u64.
    pub fn pack_u64(
        &mut self,
        values: &HashMap<String, u64>,
        obfuscate: bool,
    ) -> Result<PackedU64, ObfuskeyError> {
        self.schema.validate_values_u64(values)?;

        let mut packed: u64 = 0;
        for (name, info) in self.schema.field_info() {
            let value = values[name];
            packed |= value << info.shift;
        }

        if obfuscate {
            let obfuskey = self
                .obfuskey
                .as_mut()
                .ok_or_else(|| ObfuskeyError::ValueError("No obfuskey instance set.".to_string()))?;
            let key = obfuskey.get_key_u64(packed)?;
            Ok(PackedU64::Key(key))
        } else {
            Ok(PackedU64::Int(packed))
        }
    }

    /// Unpack a u64 or obfuscated key into u64 field values.
    pub fn unpack_u64(
        &mut self,
        data: UnpackDataU64<'_>,
        obfuscated: bool,
    ) -> Result<HashMap<String, u64>, ObfuskeyError> {
        let packed: u64 = if obfuscated {
            let key = match data {
                UnpackDataU64::Key(k) => k,
                UnpackDataU64::Int(_) => {
                    return Err(ObfuskeyError::ValueError(
                        "Expected a string key for obfuscated data.".to_string(),
                    ));
                }
            };
            let obfuskey = self
                .obfuskey
                .as_mut()
                .ok_or_else(|| ObfuskeyError::ValueError("No obfuskey instance set.".to_string()))?;
            obfuskey.get_value_u64(key)?
        } else {
            match data {
                UnpackDataU64::Int(i) => i,
                UnpackDataU64::Key(_) => {
                    return Err(ObfuskeyError::ValueError(
                        "Expected an integer for non-obfuscated data.".to_string(),
                    ));
                }
            }
        };

        let mut result = HashMap::new();
        for field in self.schema.definition() {
            let info = self.schema.get_field_info(&field.name).unwrap();
            let mask = if info.bits >= 64 {
                u64::MAX
            } else {
                (1u64 << info.bits) - 1
            };
            let value = (packed >> info.shift) & mask;
            result.insert(field.name.clone(), value);
        }

        Ok(result)
    }

    /// Pack u64 field values into bytes.
    pub fn pack_bytes_u64(
        &self,
        values: &HashMap<String, u64>,
        byteorder: ByteOrder,
    ) -> Result<Vec<u8>, ObfuskeyError> {
        self.schema.validate_values_u64(values)?;

        let mut packed: u64 = 0;
        for (name, info) in self.schema.field_info() {
            let value = values[name];
            packed |= value << info.shift;
        }

        let byte_length = ((self.schema.total_bits() + 7) / 8) as usize;
        let all_bytes = packed.to_be_bytes();
        let mut bytes = all_bytes[8 - byte_length..].to_vec();

        if byteorder == ByteOrder::Little {
            bytes.reverse();
        }

        Ok(bytes)
    }

    /// Unpack bytes into u64 field values.
    pub fn unpack_bytes_u64(
        &self,
        data: &[u8],
        byteorder: ByteOrder,
    ) -> Result<HashMap<String, u64>, ObfuskeyError> {
        let expected_len = ((self.schema.total_bits() + 7) / 8) as usize;
        if data.len() != expected_len {
            return Err(ObfuskeyError::ValueError(format!(
                "Expected {} bytes, got {}.",
                expected_len,
                data.len()
            )));
        }

        let bytes = if byteorder == ByteOrder::Little {
            let mut reversed = data.to_vec();
            reversed.reverse();
            reversed
        } else {
            data.to_vec()
        };

        let mut padded = [0u8; 8];
        padded[8 - bytes.len()..].copy_from_slice(&bytes);
        let packed = u64::from_be_bytes(padded);

        let mut result = HashMap::new();
        for field in self.schema.definition() {
            let info = self.schema.get_field_info(&field.name).unwrap();
            let mask = if info.bits >= 64 {
                u64::MAX
            } else {
                (1u64 << info.bits) - 1
            };
            let value = (packed >> info.shift) & mask;
            result.insert(field.name.clone(), value);
        }

        Ok(result)
    }

    // =========================================================================
    // BigUint API — for schemas that exceed 64 bits total
    // =========================================================================

    /// Pack BigUint field values.
    pub fn pack_big(
        &mut self,
        values: &HashMap<String, BigUint>,
        obfuscate: bool,
    ) -> Result<PackedBig, ObfuskeyError> {
        self.schema.validate_values_big(values)?;

        let mut packed = BigUint::zero();
        for (name, info) in self.schema.field_info() {
            let value = &values[name];
            packed |= value << info.shift;
        }

        if obfuscate {
            let obfuskey = self
                .obfuskey
                .as_mut()
                .ok_or_else(|| ObfuskeyError::ValueError("No obfuskey instance set.".to_string()))?;
            let key = obfuskey.get_key_big(&packed)?;
            Ok(PackedBig::Key(key))
        } else {
            Ok(PackedBig::Int(packed))
        }
    }

    /// Unpack a BigUint or obfuscated key into BigUint field values.
    pub fn unpack_big(
        &mut self,
        data: UnpackDataBig<'_>,
        obfuscated: bool,
    ) -> Result<HashMap<String, BigUint>, ObfuskeyError> {
        let packed = if obfuscated {
            let key = match data {
                UnpackDataBig::Key(k) => k,
                UnpackDataBig::Int(_) => {
                    return Err(ObfuskeyError::ValueError(
                        "Expected a string key for obfuscated data.".to_string(),
                    ));
                }
            };
            let obfuskey = self
                .obfuskey
                .as_mut()
                .ok_or_else(|| ObfuskeyError::ValueError("No obfuskey instance set.".to_string()))?;
            obfuskey.get_value_big(key)?
        } else {
            match data {
                UnpackDataBig::Int(i) => i,
                UnpackDataBig::Key(_) => {
                    return Err(ObfuskeyError::ValueError(
                        "Expected an integer for non-obfuscated data.".to_string(),
                    ));
                }
            }
        };

        let mut result = HashMap::new();
        for field in self.schema.definition() {
            let info = self.schema.get_field_info(&field.name).unwrap();
            let mask = (BigUint::one() << info.bits) - BigUint::one();
            let value = (&packed >> info.shift) & mask;
            result.insert(field.name.clone(), value);
        }

        Ok(result)
    }

    /// Pack BigUint field values into bytes.
    pub fn pack_bytes_big(
        &mut self,
        values: &HashMap<String, BigUint>,
        byteorder: ByteOrder,
    ) -> Result<Vec<u8>, ObfuskeyError> {
        self.schema.validate_values_big(values)?;

        let mut packed = BigUint::zero();
        for (name, info) in self.schema.field_info() {
            let value = &values[name];
            packed |= value << info.shift;
        }

        let byte_length = ((self.schema.total_bits() + 7) / 8) as usize;
        let mut bytes = packed.to_bytes_be();

        if bytes.len() < byte_length {
            let mut padded = vec![0u8; byte_length - bytes.len()];
            padded.extend_from_slice(&bytes);
            bytes = padded;
        }

        if byteorder == ByteOrder::Little {
            bytes.reverse();
        }

        Ok(bytes)
    }

    /// Unpack bytes into BigUint field values.
    pub fn unpack_bytes_big(
        &self,
        data: &[u8],
        byteorder: ByteOrder,
    ) -> Result<HashMap<String, BigUint>, ObfuskeyError> {
        let expected_len = ((self.schema.total_bits() + 7) / 8) as usize;
        if data.len() != expected_len {
            return Err(ObfuskeyError::ValueError(format!(
                "Expected {} bytes, got {}.",
                expected_len,
                data.len()
            )));
        }

        let bytes = if byteorder == ByteOrder::Little {
            let mut reversed = data.to_vec();
            reversed.reverse();
            reversed
        } else {
            data.to_vec()
        };

        let packed = BigUint::from_bytes_be(&bytes);

        let mut result = HashMap::new();
        for field in self.schema.definition() {
            let info = self.schema.get_field_info(&field.name).unwrap();
            let mask = (BigUint::one() << info.bits) - BigUint::one();
            let value = (&packed >> info.shift) & mask;
            result.insert(field.name.clone(), value);
        }

        Ok(result)
    }
}

// =============================================================================
// Result types
// =============================================================================

pub enum PackedU64 {
    Int(u64),
    Key(String),
}

pub enum PackedBig {
    Int(BigUint),
    Key(String),
}

pub enum UnpackDataU64<'a> {
    Int(u64),
    Key(&'a str),
}

pub enum UnpackDataBig<'a> {
    Int(BigUint),
    Key(&'a str),
}
