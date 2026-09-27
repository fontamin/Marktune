use crate::config::{Config, ControlsCfg};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cgj,
    Control { group: Group, axis: Axis, sign: i32 },
}

#[derive(Debug, Clone, Copy)]
pub struct GroupControls {
    pub x: char,
    pub x_neg: char,
    pub y: char,
    pub y_neg: char,
}

impl From<ControlsCfg> for GroupControls {
    fn from(c: ControlsCfg) -> Self {
        Self {
            x: c.x,
            x_neg: c.x_neg,
            y: c.y,
            y_neg: c.y_neg,
        }
    }
}

impl GroupControls {
    pub fn glyph(&self, axis: Axis, sign: i32) -> char {
        match (axis, sign >= 0) {
            (Axis::X, true) => self.x,
            (Axis::X, false) => self.x_neg,
            (Axis::Y, true) => self.y,
            (Axis::Y, false) => self.y_neg,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TableError {
    Duplicate(char),
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableError::Duplicate(ch) => write!(f, "repeated charachter in table: {ch:?}"),
        }
    }
}

impl std::error::Error for TableError {}

#[derive(Debug, Clone)]
pub struct CharTable {
    map: HashMap<char, Kind>,
    top: GroupControls,
    bottom: GroupControls,
    cgj: char,
}

impl CharTable {
    pub fn from_config(cfg: &Config) -> Result<Self, TableError> {
        let mut map = HashMap::new();

        fn put(map: &mut HashMap<char, Kind>, ch: char, k: Kind) -> Result<(), TableError> {
            if map.insert(ch, k).is_some() {
                Err(TableError::Duplicate(ch))
            } else {
                Ok(())
            }
        }

        put(&mut map, cfg.cgj, Kind::Cgj)?;

        for (group, g) in [
            (Group::Top, &cfg.groups.top),
            (Group::Bottom, &cfg.groups.bottom),
        ] {
            let c = g.controls;
            put(&mut map, c.x, Kind::Control { group, axis: Axis::X, sign: 1 })?;
            put(&mut map, c.x_neg, Kind::Control { group, axis: Axis::X, sign: -1 })?;
            put(&mut map, c.y, Kind::Control { group, axis: Axis::Y, sign: 1 })?;
            put(&mut map, c.y_neg, Kind::Control { group, axis: Axis::Y, sign: -1 })?;
        }

        Ok(Self {
            map,
            top: cfg.groups.top.controls.into(),
            bottom: cfg.groups.bottom.controls.into(),
            cgj: cfg.cgj,
        })
    }

    pub fn classify(&self, ch: char) -> Option<Kind> {
        self.map.get(&ch).copied()
    }

    pub fn controls(&self, g: Group) -> &GroupControls {
        match g {
            Group::Top => &self.top,
            Group::Bottom => &self.bottom,
        }
    }

    pub fn cgj(&self) -> char {
        self.cgj
    }
}
