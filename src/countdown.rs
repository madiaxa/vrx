//! Small, validated countdown templates; GUI notation stays configurable.
use crate::{Result, config::Config};
use std::time::Duration;
const TOKENS: [&str; 4] = ["minutes", "seconds", "seconds:02", "total_seconds"];
pub fn validate(template: &str) -> Result<()> {
    if template.len() > 256 {
        return Err("Countdown formats must be at most 256 bytes".into());
    }
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        if rest.get(..open).is_some_and(|s| s.contains('}')) {
            return Err("Unmatched countdown brace".into());
        }
        let tail = rest.get(open + 1..).ok_or("Invalid countdown format")?;
        let end = tail
            .find('}')
            .ok_or("Close each countdown placeholder with }")?;
        let token = tail.get(..end).ok_or("Invalid placeholder")?;
        if !TOKENS.contains(&token) {
            return Err("Use {minutes}, {seconds}, {seconds:02}, or {total_seconds}".into());
        }
        rest = tail.get(end + 1..).ok_or("Invalid countdown format")?;
    }
    if rest.contains('}') {
        return Err("Unmatched countdown brace".into());
    }
    Ok(())
}
pub fn remaining(duration: Duration, c: &Config) -> String {
    let seconds = duration
        .as_secs()
        .saturating_add(u64::from(duration.subsec_nanos() > 0));
    let template = if seconds < 60 {
        &c.countdown_short
    } else {
        &c.countdown_long
    };
    let default = if seconds < 60 {
        "{seconds}s"
    } else {
        "{minutes}:{seconds:02}"
    };
    let template = if validate(template).is_ok() {
        template.as_str()
    } else {
        default
    };
    let n = c.notation();
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(rest.get(..open).unwrap_or(""));
        let tail = rest.get(open + 1..).unwrap_or("");
        let end = tail.find('}').unwrap_or(tail.len());
        let value = match tail.get(..end).unwrap_or("") {
            "minutes" => n.integer(i128::from(seconds / 60)),
            "total_seconds" => n.integer(i128::from(seconds)),
            "seconds:02" => n.padded(i128::from(seconds % 60), 2),
            _ => n.integer(i128::from(seconds % 60)),
        };
        out.push_str(&value);
        rest = tail.get(end.saturating_add(1)..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdown_formats_padding_and_alphabets() -> Result<()> {
        let mut c = Config::default();
        assert_eq!(remaining(Duration::from_secs(6), &c), "6s");
        assert_eq!(remaining(Duration::from_secs(304), &c), "5:04");
        assert_eq!(remaining(Duration::from_millis(500), &c), "1s");
        c.countdown_short = "{seconds} seconds".into();
        c.countdown_long = "{minutes} minutes {seconds} seconds".into();
        assert_eq!(
            remaining(Duration::from_secs(304), &c),
            "5 minutes 4 seconds"
        );
        c.countdown_long = "{minutes}:{seconds:02}".into();
        c.radix = 12;
        assert_eq!(remaining(Duration::from_secs(610), &c), "↊:0↊");
        c.numeral_alphabet = "abcdefghij↊↋ABCDEFGHIJKLMNOPQRSTUVWXYZklmnopqrstuv0123456789".into();
        crate::numerals::validate_alphabet(&c.numeral_alphabet, 12, 60)?;
        assert_eq!(remaining(Duration::from_secs(304), &c), "f:ae");
        assert!(validate("{invalid}").is_err());
        assert!(validate("bad}").is_err());
        Ok(())
    }
}
