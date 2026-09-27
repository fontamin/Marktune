use crate::table::{Axis, CharTable, Group, GroupControls, Kind};

const MAX_SCAN: usize = 64;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AxisState {
    pub x: i32,
    pub y: i32,
}

impl AxisState {
    pub fn apply(&mut self, axis: Axis, sign: i32) {
        match axis {
            Axis::X => self.x += sign,
            Axis::Y => self.y += sign,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0 && self.y == 0
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GroupState {
    pub state: AxisState,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub base: Option<char>,
    pub top: GroupState,
    pub bottom: GroupState,
    pub consumed: usize,
}

impl Cluster {
    pub fn group_mut(&mut self, g: Group) -> &mut GroupState {
        match g {
            Group::Top => &mut self.top,
            Group::Bottom => &mut self.bottom,
        }
    }

    pub fn group(&self, g: Group) -> &GroupState {
        match g {
            Group::Top => &self.top,
            Group::Bottom => &self.bottom,
        }
    }
}

/// اسکن معکوس از `cursor` تا رسیدن به حرف پایه.
pub fn parse(text: &[char], cursor: usize, tbl: &CharTable) -> Cluster {
    let mut c = Cluster::default();
    let mut i = cursor.min(text.len());
    let mut steps = 0usize;

    while i > 0 && steps < MAX_SCAN {
        let ch = text[i - 1];
        let kind = tbl.classify(ch);
        i -= 1;
        steps += 1;

        match kind {
            Some(Kind::Cgj) => {}
            Some(Kind::Control { group, axis, sign }) => {
                c.group_mut(group).state.apply(axis, sign);
            }
            None => {
                c.base = Some(ch);
                break;
            }
        }
    }

    c.consumed = steps;
    c
}

pub fn serialize(c: &Cluster, tbl: &CharTable) -> String {
    let mut out = String::new();
    if let Some(base) = c.base {
        out.push(base);
    }

    emit_group(&mut out, &c.bottom, tbl.controls(Group::Bottom), tbl.cgj());
    emit_group(&mut out, &c.top, tbl.controls(Group::Top), tbl.cgj());

    out
}

fn emit_group(out: &mut String, g: &GroupState, ctl: &GroupControls, cgj: char) {
    if g.state.is_zero() {
        return;
    }
    out.push(cgj);
    emit_axis(out, ctl, Axis::X, g.state.x);
    emit_axis(out, ctl, Axis::Y, g.state.y);
}

fn emit_axis(out: &mut String, ctl: &GroupControls, axis: Axis, n: i32) {
    if n == 0 {
        return;
    }
    let glyph = ctl.glyph(axis, n);
    for _ in 0..n.abs() {
        out.push(glyph);
    }
}
