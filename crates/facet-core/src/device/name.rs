//! Renaming the cube: when the Name row may be edited, what a typed name becomes, and what the app says about it.
//!
//! The cube stores its name as at most [`MAXIMUM_LENGTH`] ASCII characters, sent with `0x15`. Nothing reads the name
//! back in answer to the write: the cube reports it as its GAP name on the next connection.

/// The most characters the cube stores in its name.
pub const MAXIMUM_LENGTH: usize = 18;

/// Why the Name row cannot be edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameRefusal {
    NotPaired,
    NotConnected,
    /// The cube has not said what it is called, so there is nothing to rename.
    NameUnknown,
}

impl RenameRefusal {
    /// What the Name row says when it cannot be edited.
    pub fn help(self) -> &'static str {
        match self {
            RenameRefusal::NotPaired => "No TimeFlip is paired, so there is no name to change.",
            RenameRefusal::NotConnected => "The TimeFlip is not connected, so this cannot be changed.",
            RenameRefusal::NameUnknown => {
                "The TimeFlip has not said what it is called yet, so there is nothing to rename."
            }
        }
    }
}

/// Why the Name row cannot be edited now, or `None` when it can. Checked in the order the variants are listed.
pub fn rename_refusal(
    is_cube_paired: bool,
    is_cube_connected: bool,
    name: Option<&str>,
) -> Option<RenameRefusal> {
    if !is_cube_paired {
        Some(RenameRefusal::NotPaired)
    } else if !is_cube_connected {
        Some(RenameRefusal::NotConnected)
    } else if name.is_none_or(|name| name.trim().is_empty()) {
        Some(RenameRefusal::NameUnknown)
    } else {
        None
    }
}

/// Why a typed name cannot be sent to the cube.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameProblem {
    /// More than [`MAXIMUM_LENGTH`] characters, with how many there were.
    TooLong(usize),
    /// A character outside printable ASCII.
    UnwritableCharacters,
    /// The cube did not take the write, with the reason.
    WriteFailed(String),
}

const NOT_OUR_RULE: &str =
    "This is a limit built into the device by TimeFlip, not something this app has decided.";
const ALLOWANCE: &str =
    "Names can be up to 18 characters long, using letters, numbers, spaces and ordinary punctuation.";

impl NameProblem {
    pub fn title(&self) -> &'static str {
        match self {
            NameProblem::TooLong(_) | NameProblem::UnwritableCharacters => {
                "The TimeFlip cannot store that name"
            }
            NameProblem::WriteFailed(_) => "The TimeFlip did not take the new name",
        }
    }

    pub fn message(&self) -> String {
        match self {
            NameProblem::TooLong(count) => format!(
                "The TimeFlip has room for only {MAXIMUM_LENGTH} characters in its name, and that one is {count}.\n\n\
                 {NOT_OUR_RULE} {ALLOWANCE}"
            ),
            NameProblem::UnwritableCharacters => format!(
                "The TimeFlip can only store plain, unaccented text in its name, so emoji, accented letters and \
                 other symbols cannot be sent to it at all.\n\n{NOT_OUR_RULE} {ALLOWANCE}"
            ),
            NameProblem::WriteFailed(reason) => format!(
                "The new name could not be sent to the device ({reason}), so it is still called what it was called \
                 before.\n\nCheck that it is connected and try again."
            ),
        }
    }
}

/// What a name typed into the Name row comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameDecision {
    /// Empty, or the name the cube already has: nothing is sent.
    Ignore,
    Refuse(NameProblem),
    /// Send this name, trimmed of surrounding whitespace.
    Write(String),
}

/// Whether `character` is one the cube can store: printable ASCII, space to tilde.
pub fn is_writable(character: char) -> bool {
    (' '..='~').contains(&character)
}

/// What `typed` comes to against the cube's `current` name. Surrounding whitespace is trimmed and interior spaces
/// kept; an unwritable character is refused before the length is.
pub fn decide(typed: &str, current: Option<&str>) -> NameDecision {
    let name = typed.trim();
    if name.is_empty() || Some(name) == current.map(str::trim) {
        return NameDecision::Ignore;
    }
    if !name.chars().all(is_writable) {
        return NameDecision::Refuse(NameProblem::UnwritableCharacters);
    }
    let count = name.chars().count();
    if count > MAXIMUM_LENGTH {
        return NameDecision::Refuse(NameProblem::TooLong(count));
    }
    NameDecision::Write(name.to_string())
}

/// What the app says once the cube has taken a new name: it keeps advertising its old one in a scan, and the
/// platform can go on reporting `previous` until the next connection.
pub fn rename_lag_notice(name: &str, previous: Option<&str>) -> String {
    let previous =
        previous.map_or("the old name".to_string(), |previous| format!("\u{201c}{previous}\u{201d}"));
    format!(
        "The TimeFlip is now called \u{201c}{name}\u{201d}, and this app will go on calling it that. Elsewhere it will \
         take a while to catch up, and neither half of that is something this app can change: the device goes on \
         advertising its original TimeFlip name in a Bluetooth scan permanently, and the system can report {previous} \
         until the next time it connects to it."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_opens_only_with_a_connected_cube_that_has_said_its_name() {
        assert_eq!(rename_refusal(false, false, None), Some(RenameRefusal::NotPaired));
        assert_eq!(rename_refusal(true, false, Some("TimeFlip v2.0")), Some(RenameRefusal::NotConnected));
        assert_eq!(rename_refusal(true, true, Some(" ")), Some(RenameRefusal::NameUnknown));
        assert_eq!(rename_refusal(true, true, Some("TimeFlip v2.0")), None);
    }

    #[test]
    fn the_limit_is_the_specs_eighteen_and_nothing_is_stripped() {
        assert_eq!(decide("  Facet cube ", Some("TimeFlip v2.0")), NameDecision::Write("Facet cube".into()));
        assert_eq!(decide("abcdefghijklmnopqr", None), NameDecision::Write("abcdefghijklmnopqr".into()));
        assert_eq!(decide("abcdefghijklmnopqrs", None), NameDecision::Refuse(NameProblem::TooLong(19)));
        assert_eq!(decide("Cube \u{1F3B2}", None), NameDecision::Refuse(NameProblem::UnwritableCharacters));
        assert_eq!(decide("Caf\u{e9}", None), NameDecision::Refuse(NameProblem::UnwritableCharacters));
        assert_eq!(decide("tab\there", None), NameDecision::Refuse(NameProblem::UnwritableCharacters));
    }

    #[test]
    fn an_empty_name_or_the_same_one_sends_nothing() {
        assert_eq!(decide("   ", Some("TimeFlip v2.0")), NameDecision::Ignore);
        assert_eq!(decide("TimeFlip v2.0 ", Some("TimeFlip v2.0")), NameDecision::Ignore);
    }

    #[test]
    fn the_refusals_say_the_limit_is_the_devices() {
        let message = NameProblem::TooLong(19).message();
        assert!(message.contains("18 characters") && message.contains("not something this app has decided"));
        assert!(NameProblem::UnwritableCharacters.message().contains("The TimeFlip can only store plain"));
        assert!(rename_lag_notice("Facet cube", Some("TimeFlip v2.0")).contains("advertising"));
    }
}
