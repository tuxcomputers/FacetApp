//! What the menu bar says beside its icon: the category being timed and today's figure, in the colour that says whose
//! clock it is, and the words a screen reader is given for it.

/// A colour in the menu bar line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColour {
    /// The system's own text colour.
    Ordinary,
    /// Timing by hand.
    ByHand,
    /// Following a cube that can be heard.
    Cube,
    /// Following a cube that cannot be heard now.
    Unreachable,
    /// The category on show has spent its daily limit, or the battery is low on the lit half of a blink.
    Spent,
}

impl StatusColour {
    /// The colour as `#rrggbb`, or `None` for the system's own.
    pub fn hex(self) -> Option<&'static str> {
        match self {
            StatusColour::Ordinary => None,
            StatusColour::ByHand => Some("#00c7d9"),
            StatusColour::Cube => Some("#34c759"),
            StatusColour::Unreachable => Some("#ffcc00"),
            StatusColour::Spent => Some("#ff3b30"),
        }
    }

    /// The word the trace uses for it.
    pub fn name(self) -> &'static str {
        match self {
            StatusColour::Ordinary => "ordinary",
            StatusColour::ByHand => "cyan",
            StatusColour::Cube => "green",
            StatusColour::Unreachable => "yellow",
            StatusColour::Spent => "red",
        }
    }
}

/// The line the menu bar shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    /// The category's name, `Facet` with nothing timed, or `Connecting...` while a launch reaches its cube.
    pub name: String,
    /// Today's figure for the category, `None` with nothing timed.
    pub figure: Option<String>,
    pub name_colour: StatusColour,
    pub figure_colour: StatusColour,
    /// What a screen reader is told, the name first and the app's own name last.
    pub spoken: String,
}

impl StatusLine {
    /// The name and the figure as one string, the way a plain-text label shows them.
    pub fn text(&self) -> String {
        match &self.figure {
            Some(figure) => format!("{} {figure}", self.name),
            None => self.name.clone(),
        }
    }

    /// The colours in the trace's words: `name <colour>, figure <colour>`.
    pub fn colour_description(&self) -> String {
        format!("name {}, figure {}", self.name_colour.name(), self.figure_colour.name())
    }
}

/// What is being timed, as the menu bar needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timed {
    pub category: String,
    /// Today's figure, already formatted.
    pub figure: String,
    /// Whether the clock is stopped: a paused segment, or a manual clock on pause.
    pub is_paused: bool,
    pub is_limit_reached: bool,
}

/// The facts the line is built from, each read at the point of use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusFacts {
    /// What is being timed, `None` when nothing is.
    pub timed: Option<Timed>,
    /// Whether the app is following a cube rather than timing by hand.
    pub is_following_cube: bool,
    pub is_cube_connected: bool,
    /// A paired launch that has not reached its cube yet.
    pub is_connecting: bool,
    pub is_cube_locked: bool,
    pub is_battery_low: bool,
    /// The lit half of the low battery blink.
    pub is_blink_on: bool,
}

/// The app's own name, shown when nothing is being timed and said last in every spoken line.
pub const APP_LABEL: &str = "Facet";
/// What the line says while a paired launch reaches its cube.
pub const CONNECTING: &str = "Connecting...";

/// The line for `facts`.
pub fn line(facts: &StatusFacts) -> StatusLine {
    if facts.is_connecting {
        return StatusLine {
            name: CONNECTING.to_string(),
            figure: None,
            name_colour: StatusColour::Ordinary,
            figure_colour: StatusColour::Ordinary,
            spoken: format!("{CONNECTING}, {APP_LABEL}"),
        };
    }
    let flash = facts.is_battery_low.then_some(if facts.is_blink_on {
        StatusColour::Spent
    } else {
        StatusColour::Ordinary
    });
    let mut spoken: Vec<String> = Vec::new();
    let Some(timed) = &facts.timed else {
        spoken.push(APP_LABEL.to_string());
        if facts.is_cube_locked {
            spoken.push("device locked".to_string());
        }
        if facts.is_battery_low {
            spoken.push("low battery".to_string());
        }
        return StatusLine {
            name: APP_LABEL.to_string(),
            figure: None,
            name_colour: flash.unwrap_or(StatusColour::Ordinary),
            figure_colour: StatusColour::Ordinary,
            spoken: spoken.join(", "),
        };
    };
    let (own, figure_colour) = if !facts.is_following_cube {
        (
            StatusColour::ByHand,
            if timed.is_limit_reached { StatusColour::Spent } else { StatusColour::ByHand },
        )
    } else if facts.is_cube_connected {
        (StatusColour::Cube, if timed.is_limit_reached { StatusColour::Spent } else { StatusColour::Cube })
    } else {
        (StatusColour::Unreachable, StatusColour::Unreachable)
    };
    let name_colour =
        if facts.is_following_cube && !facts.is_cube_connected { own } else { flash.unwrap_or(own) };
    spoken.push(timed.category.clone());
    spoken.push(if timed.is_paused { "paused" } else { "running" }.to_string());
    if facts.is_following_cube && !facts.is_cube_connected {
        spoken.push("device unreachable".to_string());
    }
    spoken.push(timed.figure.clone());
    if facts.is_cube_locked {
        spoken.push("device locked".to_string());
    }
    if timed.is_limit_reached {
        spoken.push("daily limit reached".to_string());
    }
    if facts.is_battery_low {
        spoken.push("low battery".to_string());
    }
    spoken.push(APP_LABEL.to_string());
    StatusLine {
        name: timed.category.clone(),
        figure: Some(timed.figure.clone()),
        name_colour,
        figure_colour,
        spoken: spoken.join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(timed: Option<Timed>) -> StatusFacts {
        StatusFacts {
            timed,
            is_following_cube: false,
            is_cube_connected: false,
            is_connecting: false,
            is_cube_locked: false,
            is_battery_low: false,
            is_blink_on: false,
        }
    }

    fn break_at(figure: &str) -> Timed {
        Timed { category: "Break".into(), figure: figure.into(), is_paused: false, is_limit_reached: false }
    }

    #[test]
    fn nothing_timed_says_facet_and_connecting_says_so() {
        let idle = line(&facts(None));
        assert_eq!((idle.text(), idle.name_colour), ("Facet".to_string(), StatusColour::Ordinary));
        let connecting = line(&StatusFacts { is_connecting: true, ..facts(Some(break_at("0:01:00"))) });
        assert_eq!(connecting.text(), "Connecting...");
    }

    #[test]
    fn the_colour_says_whose_clock_it_is() {
        let by_hand = line(&facts(Some(break_at("0:01:00"))));
        assert_eq!(by_hand.text(), "Break 0:01:00");
        assert_eq!(by_hand.colour_description(), "name cyan, figure cyan");
        let cube = StatusFacts {
            is_following_cube: true,
            is_cube_connected: true,
            ..facts(Some(break_at("0:01:00")))
        };
        assert_eq!(line(&cube).colour_description(), "name green, figure green");
        let unheard = StatusFacts { is_cube_connected: false, ..cube.clone() };
        assert_eq!(line(&unheard).colour_description(), "name yellow, figure yellow");
        assert!(line(&unheard).spoken.contains("device unreachable"));
        let spent = StatusFacts {
            timed: Some(Timed { is_limit_reached: true, ..break_at("1:00:00") }),
            ..cube.clone()
        };
        assert_eq!(line(&spent).colour_description(), "name green, figure red");
        assert!(line(&spent).spoken.contains("daily limit reached"));
    }

    #[test]
    fn a_low_battery_flashes_the_name() {
        let cube = StatusFacts {
            is_following_cube: true,
            is_cube_connected: true,
            is_battery_low: true,
            is_blink_on: true,
            ..facts(Some(break_at("0:01:00")))
        };
        assert_eq!(line(&cube).name_colour, StatusColour::Spent);
        assert_eq!(
            line(&StatusFacts { is_blink_on: false, ..cube.clone() }).name_colour,
            StatusColour::Ordinary
        );
        assert!(line(&cube).spoken.ends_with("low battery, Facet"));
    }
}
