use crate::cluster::{parse, serialize, Cluster};
use crate::table::{Axis, CharTable, Group};

/// یک جایگزینی بازه‌ای
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// اندیس char (شروع، شامل)
    pub start: usize,
    /// اندیس char (پایان، انحصاری) — معمولاً برابر cursor
    pub end: usize,
    pub replacement: String,
}

/// اعمال یک دستور هات‌کی. نتیجه، جایگزینی کل cluster است.
pub fn apply_command(
    text: &[char],
    cursor: usize,
    tbl: &CharTable,
    group: Group,
    axis: Axis,
    sign: i32,
) -> Edit {
    let mut c: Cluster = parse(text, cursor, tbl);
    c.group_mut(group).state.apply(axis, sign);
    build_edit(text, cursor, &c, tbl)
}

/// بازنویسی cluster بدون تغییر وضعیت — برای نرمال‌سازی
pub fn normalize(text: &[char], cursor: usize, tbl: &CharTable) -> Edit {
    let c = parse(text, cursor, tbl);
    build_edit(text, cursor, &c, tbl)
}

fn build_edit(text: &[char], cursor: usize, c: &Cluster, tbl: &CharTable) -> Edit {
    let end = cursor.min(text.len());
    let start = end - c.consumed;
    let replacement = serialize(c, tbl);

    Edit { start, end, replacement }
}
