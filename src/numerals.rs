//! Clockette-compatible presentation notation; storage/API values stay decimal.
use crate::Result;
pub const DIGITS: &str = "0123456789↊↋ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuv";

#[derive(Debug, Clone, Copy)]
pub struct Notation(pub u8);
impl Notation {
    pub fn base(self) -> u8 {
        self.0.clamp(2, 60)
    }
    pub fn integer(self, value: i128) -> String {
        if value == 0 {
            return "0".into();
        }
        if self.base() == 3 {
            let mut value = value;
            let mut out = Vec::new();
            while value != 0 {
                let digit = value.rem_euclid(3);
                // Division before the carry avoids overflowing i128::MIN/MAX.
                value = value.div_euclid(3);
                match digit {
                    0 => out.push('0'),
                    1 => out.push('1'),
                    _ => {
                        out.push('T');
                        value += 1;
                    }
                }
            }
            return out.into_iter().rev().collect();
        }
        let mut magnitude = value.unsigned_abs();
        let mut out = Vec::new();
        while magnitude != 0 {
            if let Some(digit) = DIGITS
                .chars()
                .nth((magnitude % u128::from(self.base())) as usize)
            {
                out.push(digit);
            }
            magnitude /= u128::from(self.base());
        }
        if value < 0 {
            out.push('−');
        }
        out.into_iter().rev().collect()
    }
    /// Use the shortest fractional spelling that round-trips to the stored
    /// integer unit. This avoids long repeating-looking decimal conversions.
    pub fn scaled(self, value: u32, scale: u32) -> String {
        if scale <= 1 {
            return self.integer(i128::from(value));
        }
        let value = i128::from(value);
        let scale = i128::from(scale);
        let mut factor = 1i128;
        // A u32 scale needs at most 33 binary places. Returning once factor
        // reaches scale bounds all intermediate products well below i128::MAX.
        for places in 0..=33usize {
            let rounded = (value * factor + scale / 2) / scale;
            if (rounded * scale + factor / 2) / factor == value || factor >= scale {
                let mut digits: Vec<char> = self.integer(rounded).chars().collect();
                if places == 0 {
                    return digits.into_iter().collect();
                }
                while digits.len() <= places {
                    digits.insert(0, '0');
                }
                let point = digits.len().saturating_sub(places);
                digits.insert(point, '.');
                return digits.into_iter().collect();
            }
            factor *= i128::from(self.base());
        }
        self.integer(value)
    }
    pub fn parse_integer(self, input: &str) -> Result<i128> {
        self.parse_scaled(input, 1, false)
    }
    pub fn parse_quantity(self, input: &str, scale: u32) -> Result<i128> {
        self.parse_scaled(input, scale, true)
    }
    fn parse_scaled(self, input: &str, scale: u32, fraction: bool) -> Result<i128> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Enter a number".into());
        }
        let (negative, input) = if let Some(body) = input.strip_prefix(['-', '−']) {
            (true, body)
        } else {
            (false, input)
        };
        let mut value = 0i128;
        let mut denominator = 1i128;
        let mut after_point = false;
        let mut count = 0;
        for c in input.chars() {
            if c == '.' && fraction && !after_point {
                after_point = true;
                continue;
            }
            let digit = if self.base() == 3 {
                match c {
                    'T' | 't' => -1,
                    '0' => 0,
                    '1' => 1,
                    _ => return Err("Balanced ternary uses T, 0, 1".into()),
                }
            } else {
                let digit = DIGITS.chars().position(|d| d == c).ok_or("Unknown digit")?;
                if digit >= self.base() as usize {
                    return Err("Digit is outside the selected radix".into());
                }
                digit as i128
            };
            value = value
                .checked_mul(i128::from(self.base()))
                .and_then(|v| v.checked_add(digit))
                .ok_or("Number is too large")?;
            if after_point {
                denominator = denominator
                    .checked_mul(i128::from(self.base()))
                    .ok_or("Too many fractional digits")?;
            }
            count += 1;
        }
        if count == 0 {
            return Err("Enter a number".into());
        }
        if negative {
            value = value.checked_neg().ok_or("Number is too large")?;
        }
        let numerator = value
            .checked_mul(i128::from(scale))
            .ok_or("Number is too large")?;
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        if remainder.unsigned_abs() >= (denominator as u128).div_ceil(2) {
            quotient
                .checked_add(if numerator < 0 { -1 } else { 1 })
                .ok_or_else(|| "Number is too large".into())
        } else {
            Ok(quotient)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_base_round_trips() -> Result<()> {
        for base in 2..=60 {
            for value in [-1_000_000, -12, -1, 0, 1, 2, 10, 11, 12, 300, 10_000] {
                let n = Notation(base);
                assert_eq!(n.parse_integer(&n.integer(value))?, value);
            }
        }
        Ok(())
    }
    #[test]
    fn pitman_letters_and_balanced_ternary() {
        assert_eq!(Notation(12).integer(10), "↊");
        assert_eq!(Notation(12).integer(11), "↋");
        assert_eq!(Notation(16).integer(12), "A");
        assert_eq!(Notation(3).integer(2), "1T");
        assert_eq!(Notation(3).integer(-1), "T");
    }
    #[test]
    fn fractions_round_trip_weights() -> Result<()> {
        for base in 2..=60 {
            for value in [1000, 1150, 1300, 1500, 1800, 5000] {
                let n = Notation(base);
                assert_eq!(
                    n.parse_quantity(&n.scaled(value, 1000), 1000)?,
                    i128::from(value)
                );
            }
        }
        Ok(())
    }
    #[test]
    fn rejects_invalid_and_overflow() {
        assert!(Notation(3).parse_integer("2").is_err());
        assert!(Notation(12).parse_integer("A").is_err());
        assert!(Notation(10).parse_integer(&"9".repeat(100)).is_err());
    }
}
