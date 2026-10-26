//! Persisted portable shortcuts, independent of the window toolkit.
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Action {
    Toggle,
    Edit,
    Backward,
    Forward,
    PreviousWord,
    NextWord,
    PreviousSentence,
    NextSentence,
    PreviousParagraph,
    NextParagraph,
    SpeedUp,
    SpeedDown,
    Restart,
    Focus,
    Fullscreen,
    Personalize,
    Open,
    Paste,
    Export,
    ExitFullscreen,
}
impl Action {
    pub const ALL: [Self; 20] = [
        Self::Toggle,
        Self::Edit,
        Self::Backward,
        Self::Forward,
        Self::PreviousWord,
        Self::NextWord,
        Self::PreviousSentence,
        Self::NextSentence,
        Self::PreviousParagraph,
        Self::NextParagraph,
        Self::SpeedUp,
        Self::SpeedDown,
        Self::Restart,
        Self::Focus,
        Self::Fullscreen,
        Self::Personalize,
        Self::Open,
        Self::Paste,
        Self::Export,
        Self::ExitFullscreen,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Play / pause",
            Self::Edit => "Edit current text",
            Self::Backward => "Jump backward",
            Self::Forward => "Jump forward",
            Self::PreviousWord => "Previous word",
            Self::NextWord => "Next word",
            Self::PreviousSentence => "Previous sentence",
            Self::NextSentence => "Next sentence",
            Self::PreviousParagraph => "Previous paragraph",
            Self::NextParagraph => "Next paragraph",
            Self::SpeedUp => "Increase WPM",
            Self::SpeedDown => "Decrease WPM",
            Self::Restart => "Restart",
            Self::Focus => "Focus mode",
            Self::Fullscreen => "Fullscreen",
            Self::Personalize => "Personalize menu",
            Self::Open => "Open file",
            Self::Paste => "Paste text",
            Self::Export => "Export",
            Self::ExitFullscreen => "Exit fullscreen",
        }
    }
    pub fn repeatable(self) -> bool {
        matches!(
            self,
            Self::Backward
                | Self::Forward
                | Self::PreviousWord
                | Self::NextWord
                | Self::PreviousSentence
                | Self::NextSentence
                | Self::PreviousParagraph
                | Self::NextParagraph
                | Self::SpeedUp
                | Self::SpeedDown
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
    pub command: bool,
}
impl Chord {
    pub fn parse(input: &str) -> Result<Self> {
        let mut chord = Self {
            key: String::new(),
            ctrl: false,
            alt: false,
            shift: false,
            logo: false,
            command: false,
        };
        for part in input.split('+').map(str::trim) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => chord.ctrl = true,
                "alt" | "option" => chord.alt = true,
                "shift" => chord.shift = true,
                "cmd" | "command" | "super" | "meta" => chord.logo = true,
                "cmdorctrl" => chord.command = true,
                "" => return Err("Choose a key, or use Clear to unbind".into()),
                _ if chord.key.is_empty() => chord.key = canonical_key(part),
                _ => return Err("A shortcut has exactly one main key".into()),
            }
        }
        if chord.key.is_empty() {
            return Err("Choose a non-modifier key".into());
        }
        Ok(chord)
    }
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.command {
            parts.push("CmdOrCtrl");
        }
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.logo {
            parts.push("Cmd");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }
    pub fn matches(&self, input: &Self, mac: bool) -> bool {
        self.key.eq_ignore_ascii_case(&input.key)
            && self.shift == input.shift
            && self.alt == input.alt
            && (self.ctrl || (self.command && !mac)) == input.ctrl
            && (self.logo || (self.command && mac)) == input.logo
    }
}
pub fn canonical_key(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        " " | "space" => "Space".into(),
        "esc" | "escape" => "Escape".into(),
        "arrowleft" | "left" | "←" => "Left".into(),
        "arrowright" | "right" | "→" => "Right".into(),
        "arrowup" | "up" | "↑" => "Up".into(),
        "arrowdown" | "down" | "↓" => "Down".into(),
        "," | "comma" => "Comma".into(),
        "+" | "plus" => "Plus".into(),
        ";" | "semicolon" => "Semicolon".into(),
        other if other.chars().count() == 1 => key.to_uppercase(),
        _ => {
            let mut chars = key.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keymap {
    #[serde(deserialize_with = "deserialize_bindings")]
    pub bindings: BTreeMap<Action, String>,
}
// Older saved maps included a Maximize action. Ignore only that retired entry
// so removing its UI/action does not discard the user's other preferences.
fn deserialize_bindings<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<Action, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let stored = BTreeMap::<String, String>::deserialize(deserializer)?;
    let mut bindings = BTreeMap::new();
    for (name, chord) in stored {
        if name == "Maximize" {
            continue;
        }
        let action =
            Action::deserialize(serde::de::value::StringDeserializer::<D::Error>::new(name))?;
        bindings.insert(action, chord);
    }
    Ok(bindings)
}

impl Default for Keymap {
    fn default() -> Self {
        use Action::*;
        let pairs = [
            (Toggle, "Space"),
            (Edit, "CmdOrCtrl+N"),
            (Backward, "Left"),
            (Forward, "Right"),
            (PreviousWord, "Shift+Left"),
            (NextWord, "Shift+Right"),
            (PreviousSentence, "CmdOrCtrl+Left"),
            (NextSentence, "CmdOrCtrl+Right"),
            (PreviousParagraph, "Alt+Left"),
            (NextParagraph, "Alt+Right"),
            (SpeedUp, "Up"),
            (SpeedDown, "Down"),
            (Restart, "R"),
            (Focus, "H"),
            (Fullscreen, "F11"),
            (Personalize, "Escape; CmdOrCtrl+Comma"),
            (Open, "CmdOrCtrl+O"),
            (Paste, "CmdOrCtrl+V"),
            (Export, "CmdOrCtrl+E"),
            (ExitFullscreen, "Shift+F11"),
        ];
        Self {
            bindings: pairs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}
impl Keymap {
    pub fn label(&self, action: Action) -> &str {
        self.bindings.get(&action).map(String::as_str).unwrap_or("")
    }
    pub fn action(&self, input: &Chord, mac: bool) -> Option<Action> {
        self.bindings.iter().find_map(|(action, text)| {
            text.split(';')
                .filter_map(|s| Chord::parse(s).ok())
                .any(|chord| chord.matches(input, mac))
                .then_some(*action)
        })
    }
    pub fn assign(&mut self, action: Action, text: &str) -> Result<()> {
        if text.trim().is_empty() {
            self.bindings.insert(action, String::new());
            return Ok(());
        }
        let chords = text
            .split(';')
            .map(Chord::parse)
            .collect::<Result<Vec<_>>>()?;
        for (other, label) in &self.bindings {
            if *other == action {
                continue;
            }
            for existing in label.split(';').filter_map(|s| Chord::parse(s).ok()) {
                if chords.iter().any(|chord| {
                    [false, true].into_iter().any(|mac| {
                        existing.key.eq_ignore_ascii_case(&chord.key)
                            && existing.shift == chord.shift
                            && existing.alt == chord.alt
                            && (existing.ctrl || (existing.command && !mac))
                                == (chord.ctrl || (chord.command && !mac))
                            && (existing.logo || (existing.command && mac))
                                == (chord.logo || (chord.command && mac))
                    })
                }) {
                    return Err(format!(
                        "That shortcut is used by {}. Clear it first.",
                        other.label()
                    ));
                }
            }
        }
        self.bindings.insert(
            action,
            chords
                .iter()
                .map(Chord::label)
                .collect::<Vec<_>>()
                .join("; "),
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_and_conflicting_shortcuts() -> Result<()> {
        let mut map = Keymap::default();
        let right = Chord::parse("Right")?;
        assert_eq!(map.action(&right, true), Some(Action::Forward));
        assert_eq!(
            map.action(&Chord::parse("Cmd+O")?, true),
            Some(Action::Open)
        );
        assert_eq!(
            map.action(&Chord::parse("Ctrl+O")?, false),
            Some(Action::Open)
        );
        assert!(map.assign(Action::Focus, "Right").is_err());
        map.assign(Action::Focus, "J")?;
        assert_eq!(map.action(&Chord::parse("J")?, false), Some(Action::Focus));
        Ok(())
    }
}
