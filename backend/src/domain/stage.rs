use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Backlog,
    Plan,
    Todo,
    InProgress,
    Review,
    Done,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::Backlog => "backlog",
            Stage::Plan => "plan",
            Stage::Todo => "todo",
            Stage::InProgress => "in_progress",
            Stage::Review => "review",
            Stage::Done => "done",
        }
    }

    pub fn all() -> &'static [Stage] {
        &[
            Stage::Backlog,
            Stage::Plan,
            Stage::Todo,
            Stage::InProgress,
            Stage::Review,
            Stage::Done,
        ]
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Stage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Stage::Backlog),
            "plan" => Ok(Stage::Plan),
            "todo" => Ok(Stage::Todo),
            "in_progress" => Ok(Stage::InProgress),
            "review" => Ok(Stage::Review),
            "done" => Ok(Stage::Done),
            _ => Err(format!("Invalid stage: {}", s)),
        }
    }
}
