use std::{cmp::Ordering, ops::Range};

use log::*;

use super::{DiffPart, DiffRange};

#[derive(Debug, Default)]
struct LineNumberMapItem {
    range: Range<usize>,
    add: usize,
    sub: usize,
    is_delete: bool,
}

#[derive(Debug, Default)]
pub struct LineNumberMap {
    items: Vec<LineNumberMapItem>,
}

impl LineNumberMap {
    /// To map old line numbers to new.
    pub fn new_new_from_old(parts: &Vec<DiffPart>) -> LineNumberMap {
        let mut items: Vec<LineNumberMapItem> = Vec::new();
        let mut add = 0;
        let mut sub = 0;
        for part in parts {
            trace!("new_new_from_old: {:?}", part);
            let old_size = part.old.len();
            let new_size = part.new.len();
            match new_size.cmp(&old_size) {
                Ordering::Equal => {}
                Ordering::Greater => {
                    let start = part.old.line_numbers.start + old_size;
                    add += new_size - old_size;
                    items.push(LineNumberMapItem {
                        range: start..start,
                        add,
                        sub,
                        is_delete: false,
                    });
                }
                Ordering::Less => {
                    let mut start = part.old.line_numbers.start + new_size;
                    items.push(LineNumberMapItem {
                        range: start..start,
                        add,
                        sub,
                        is_delete: true,
                    });
                    let delta = old_size - new_size;
                    sub += delta;
                    start += delta;
                    items.push(LineNumberMapItem {
                        range: start..start,
                        add,
                        sub,
                        is_delete: false,
                    });
                }
            }
        }
        let mut end = usize::MAX;
        for item in items.iter_mut().rev() {
            item.range.end = end;
            end = item.range.start;
        }
        LineNumberMap { items }
    }

    pub fn map(&self, old: usize) -> usize {
        let mut values = [old];
        self.apply_to_values(values.iter_mut());
        values[0]
    }

    pub fn apply_to_parts(&self, parts: &mut Vec<DiffPart>) {
        trace!("apply_to_parts: self: {self:?}");
        trace!("apply_to_parts: parts: {parts:?}");
        DiffPart::validate_ascending_parts(parts).unwrap();
        let old_ranges = parts.iter_mut().map(|part| &mut part.old);
        self.apply_to_ranges(old_ranges);
        let new_ranges = parts.iter_mut().map(|part| &mut part.new);
        self.apply_to_ranges(new_ranges);
        DiffPart::validate_ascending_parts(parts).unwrap();
    }

    pub fn apply_to_ranges<'a>(&self, ranges: impl Iterator<Item = &'a mut DiffRange>) {
        let values =
            ranges.flat_map(|lines| [&mut lines.line_numbers.start, &mut lines.line_numbers.end]);
        self.apply_to_values(values);
    }

    pub fn apply_to_values<'a>(&self, values: impl Iterator<Item = &'a mut usize>) {
        if self.items.is_empty() {
            return;
        }
        assert_eq!(self.items.last().unwrap().range.end, usize::MAX);
        let mut item_iter = self.items.iter();
        let mut item = item_iter.next().unwrap();
        let mut last_value = 0;
        let mut last_new_value = 0;
        for value in values {
            assert!(*value >= last_value);
            last_value = *value;

            if *value < item.range.start {
                continue;
            }
            while *value > item.range.end {
                item = item_iter.next().unwrap();
            }
            assert!(*value >= item.range.start);
            let new_value = if item.is_delete {
                item.range.start + item.add - item.sub
            } else if *value == usize::MAX {
                usize::MAX
            } else {
                *value + item.add - item.sub
            };
            debug!("apply_to_values: {} -> {}", *value, new_value);
            assert!(new_value >= last_new_value);
            last_new_value = new_value;
            *value = new_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_number_map_new_from_old_add() {
        let parts = vec![DiffPart {
            old: DiffRange { line_numbers: 3..3 },
            new: DiffRange { line_numbers: 3..5 },
        }];
        let map = LineNumberMap::new_new_from_old(&parts);
        // panic!("{:#?}", map);
        assert_eq!(map.map(1), 1);
        assert_eq!(map.map(2), 2);
        assert_eq!(map.map(3), 5);
        assert_eq!(map.map(4), 6);
    }

    #[test]
    fn line_number_map_new_from_old_del() {
        let parts = vec![DiffPart {
            old: DiffRange { line_numbers: 3..5 },
            new: DiffRange { line_numbers: 3..3 },
        }];
        let map = LineNumberMap::new_new_from_old(&parts);
        // panic!("{:#?}", map);
        assert_eq!(map.map(1), 1);
        assert_eq!(map.map(2), 2);
        assert_eq!(map.map(3), 3);
        assert_eq!(map.map(4), 3);
        assert_eq!(map.map(5), 3);
        assert_eq!(map.map(6), 4);
    }

    #[test]
    fn line_number_map_new_from_old_mix() {
        let parts = vec![
            DiffPart {
                old: DiffRange {
                    line_numbers: 137..137,
                },
                new: DiffRange {
                    line_numbers: 137..143,
                },
            },
            DiffPart {
                old: DiffRange {
                    line_numbers: 360..361,
                },
                new: DiffRange {
                    line_numbers: 366..366,
                },
            },
        ];
        let map = LineNumberMap::new_new_from_old(&parts);
        // panic!("{:#?}", map);
        assert_eq!(map.map(359), 365);
        assert_eq!(map.map(361), 366);
    }
}
