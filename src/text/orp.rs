use serde::{Deserialize, Serialize};

/// Returns a zero-based grapheme index, never a byte or scalar index.
pub trait OrpAlgorithm {
    fn index(&self, graphemes: usize) -> usize;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    #[default]
    Classic,
    #[serde(alias = "thirty_percent")]
    FortyPercent,
    Center,
    First,
}
impl Strategy {
    pub const ALL: [Self; 4] = [Self::Classic, Self::FortyPercent, Self::Center, Self::First];
}
impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Classic => "Classic",
            Self::FortyPercent => "40 percent",
            Self::Center => "Center",
            Self::First => "First grapheme",
        })
    }
}
impl OrpAlgorithm for Strategy {
    fn index(&self, n: usize) -> usize {
        let index = match self {
            Self::Classic => match n {
                0..=1 => 0,
                2..=5 => 1,
                6..=9 => 2,
                10..=13 => 3,
                _ => n.saturating_mul(4).div_ceil(10).saturating_sub(1),
            },
            Self::FortyPercent => n.saturating_mul(4).div_ceil(10).saturating_sub(1),
            Self::Center => n.saturating_sub(1) / 2,
            Self::First => 0,
        };
        index.min(n.saturating_sub(1))
    }
}
