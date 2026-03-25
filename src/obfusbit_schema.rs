use num_bigint::BigUint;
use num_traits::One;
use std::collections::HashMap;

use crate::error::ObfuskeyError;

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub bits: u32,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub bits: u32,
    pub shift: u32,
}

#[derive(Debug)]
pub struct ObfusbitSchema {
    definition: Vec<FieldSchema>,
    field_info: HashMap<String, FieldInfo>,
    total_bits: u32,
    maximum_value: BigUint,
}

impl ObfusbitSchema {
    pub fn new(schema: Vec<FieldSchema>) -> Result<Self, ObfuskeyError> {
        if schema.is_empty() {
            return Err(ObfuskeyError::SchemaValidationError(
                "Schema must contain at least one field.".to_string(),
            ));
        }

        let mut seen_names = std::collections::HashSet::new();
        for field in &schema {
            if field.bits == 0 {
                return Err(ObfuskeyError::SchemaValidationError(format!(
                    "Field '{}' must have a positive number of bits.",
                    field.name
                )));
            }
            if !seen_names.insert(&field.name) {
                return Err(ObfuskeyError::SchemaValidationError(format!(
                    "Duplicate field name '{}'.",
                    field.name
                )));
            }
        }

        let total_bits: u32 = schema.iter().map(|f| f.bits).sum();
        let maximum_value = (BigUint::one() << total_bits) - BigUint::one();

        // Calculate field_info in reverse order (last field = LSB)
        let mut field_info = HashMap::new();
        let mut shift: u32 = 0;

        for field in schema.iter().rev() {
            field_info.insert(
                field.name.clone(),
                FieldInfo {
                    bits: field.bits,
                    shift,
                },
            );
            shift += field.bits;
        }

        Ok(ObfusbitSchema {
            definition: schema,
            field_info,
            total_bits,
            maximum_value,
        })
    }

    pub fn definition(&self) -> &[FieldSchema] {
        &self.definition
    }

    pub fn field_info(&self) -> &HashMap<String, FieldInfo> {
        &self.field_info
    }

    pub fn get_field_info(&self, name: &str) -> Option<&FieldInfo> {
        self.field_info.get(name)
    }

    pub fn total_bits(&self) -> u32 {
        self.total_bits
    }

    pub fn maximum_value(&self) -> &BigUint {
        &self.maximum_value
    }

    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.definition.iter().map(|f| f.name.as_str())
    }

    /// Validate u64 field values against the schema.
    pub fn validate_values_u64(
        &self,
        values: &HashMap<String, u64>,
    ) -> Result<(), ObfuskeyError> {
        for field in &self.definition {
            if !values.contains_key(&field.name) {
                return Err(ObfuskeyError::ValueError(format!(
                    "Missing required field '{}'.",
                    field.name
                )));
            }
        }

        for key in values.keys() {
            if !self.field_info.contains_key(key) {
                return Err(ObfuskeyError::ValueError(format!(
                    "Unexpected field '{}'.",
                    key
                )));
            }
        }

        for (name, &value) in values {
            let info = &self.field_info[name];
            if info.bits < 64 {
                let max_field_value = (1u64 << info.bits) - 1;
                if value > max_field_value {
                    return Err(ObfuskeyError::BitOverflowError(format!(
                        "Value {} for field '{}' exceeds {} bits (max {}).",
                        value, name, info.bits, max_field_value
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate BigUint field values against the schema.
    pub fn validate_values_big(
        &self,
        values: &HashMap<String, BigUint>,
    ) -> Result<(), ObfuskeyError> {
        for field in &self.definition {
            if !values.contains_key(&field.name) {
                return Err(ObfuskeyError::ValueError(format!(
                    "Missing required field '{}'.",
                    field.name
                )));
            }
        }

        for key in values.keys() {
            if !self.field_info.contains_key(key) {
                return Err(ObfuskeyError::ValueError(format!(
                    "Unexpected field '{}'.",
                    key
                )));
            }
        }

        for (name, value) in values {
            let info = &self.field_info[name];
            let max_field_value = (BigUint::one() << info.bits) - BigUint::one();
            if *value > max_field_value {
                return Err(ObfuskeyError::BitOverflowError(format!(
                    "Value {} for field '{}' exceeds {} bits (max {}).",
                    value, name, info.bits, max_field_value
                )));
            }
        }

        Ok(())
    }
}
