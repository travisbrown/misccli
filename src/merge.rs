use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::io::{BufRead, Lines};

pub fn merge_lines<BL: BufRead, BR: BufRead>(
    left: Lines<BL>,
    right: Lines<BR>,
) -> Merge<String, Lines<BL>, Lines<BR>> {
    Merge::new(left, right)
}

pub fn merge<
    T: Ord,
    E,
    IL: IntoIterator<Item = Result<T, E>>,
    IR: IntoIterator<Item = Result<T, E>>,
>(
    left: IL,
    right: IR,
) -> Merge<T, IL::IntoIter, IR::IntoIter> {
    Merge::new(left.into_iter(), right.into_iter())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Left => "left",
            Self::Right => "right",
        })
    }
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum MergeError<T> {
    #[error(
        "unsorted pair in the {side} file at lines {} and {}: {first:?}, {second:?}",
        .index + 1,
        .index + 2
    )]
    UnsortedPair {
        side: Side,
        index: usize,
        first: T,
        second: T,
    },
}

impl<T> MergeError<T> {
    fn unsorted_pair(side: Side, index: usize, first: T, second: T) -> Self {
        Self::UnsortedPair {
            side,
            index,
            first,
            second,
        }
    }
}

type NestedResult<T, E, A> = Result<Result<A, E>, MergeError<T>>;

/// The state of one input: its iterator, position, current value, and the
/// first value read past the current one (which becomes current on advance).
struct SideState<T, I> {
    side: Side,
    iter: Option<I>,
    index: usize,
    last: Option<T>,
    pending: Option<T>,
}

impl<T: Ord, E, I: Iterator<Item = Result<T, E>>> SideState<T, I> {
    fn new(side: Side, iter: I) -> Self {
        Self {
            side,
            iter: Some(iter),
            index: 0,
            last: None,
            pending: None,
        }
    }

    fn read(&mut self) -> Result<Option<T>, E> {
        match self.iter.as_mut().and_then(Iterator::next) {
            Some(Ok(value)) => {
                self.index += 1;
                Ok(Some(value))
            }
            Some(Err(error)) => {
                self.iter = None;
                Err(error)
            }
            None => {
                self.iter = None;
                Ok(None)
            }
        }
    }

    /// Read past duplicates of `last`, returning it with the next distinct
    /// value (or `None` at the end of the input), and failing if a smaller
    /// value shows the input is unsorted.
    fn read_next(&mut self, last: T) -> NestedResult<T, E, (T, Option<T>)> {
        loop {
            match self.read() {
                Ok(Some(value)) => match last.cmp(&value) {
                    Ordering::Less => {
                        return Ok(Ok((last, Some(value))));
                    }
                    Ordering::Equal => {}
                    Ordering::Greater => {
                        return Err(MergeError::unsorted_pair(
                            self.side,
                            self.index - 2,
                            last,
                            value,
                        ));
                    }
                },
                Ok(None) => {
                    return Ok(Ok((last, None)));
                }
                Err(error) => {
                    return Ok(Err(error));
                }
            }
        }
    }

    /// Make `seed` the current value and read ahead to the next distinct one.
    fn refill(&mut self, seed: T) -> NestedResult<T, E, ()> {
        // The `?` propagates the `MergeError` layer; the read error layer is
        // handled explicitly.
        match self.read_next(seed)? {
            Ok((last, pending)) => {
                self.last = Some(last);
                self.pending = pending;
                Ok(Ok(()))
            }
            Err(error) => Ok(Err(error)),
        }
    }

    fn start(&mut self) -> NestedResult<T, E, ()> {
        match self.read() {
            Ok(Some(value)) => self.refill(value),
            Ok(None) => Ok(Ok(())),
            Err(error) => Ok(Err(error)),
        }
    }

    fn advance(&mut self) -> NestedResult<T, E, ()> {
        match self.pending.take() {
            Some(pending) => self.refill(pending),
            None => {
                self.last = None;
                Ok(Ok(()))
            }
        }
    }
}

pub struct Merge<T, IL, IR> {
    left: SideState<T, IL>,
    right: SideState<T, IR>,
    started: bool,
}

impl<T: Ord, E, IL: Iterator<Item = Result<T, E>>, IR: Iterator<Item = Result<T, E>>>
    Merge<T, IL, IR>
{
    fn new(left: IL, right: IR) -> Self {
        Self {
            left: SideState::new(Side::Left, left),
            right: SideState::new(Side::Right, right),
            started: false,
        }
    }

    /// Drop both iterators after a read error so no further input is pulled.
    fn halt(&mut self) {
        self.left.iter = None;
        self.right.iter = None;
    }
}

/// Run a side operation inside `next`, converting the two error layers of its
/// `NestedResult` into early returns of the corresponding iterator items.
macro_rules! try_side {
    ($self:ident, $side:ident, $method:ident) => {
        match $self.$side.$method() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                $self.halt();
                return Some(Ok(Err(error)));
            }
            Err(error) => {
                return Some(Err(error));
            }
        }
    };
}

impl<T: Ord, E, IL: Iterator<Item = Result<T, E>>, IR: Iterator<Item = Result<T, E>>> Iterator
    for Merge<T, IL, IR>
{
    type Item = Result<Result<T, E>, MergeError<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.started = true;
            try_side!(self, left, start);
            try_side!(self, right, start);
        }

        match (self.left.last.take(), self.right.last.take()) {
            (Some(left), Some(right)) => match left.cmp(&right) {
                Ordering::Less => {
                    try_side!(self, left, advance);
                    self.right.last = Some(right);
                    Some(Ok(Ok(left)))
                }
                Ordering::Greater => {
                    try_side!(self, right, advance);
                    self.left.last = Some(left);
                    Some(Ok(Ok(right)))
                }
                Ordering::Equal => {
                    try_side!(self, left, advance);
                    try_side!(self, right, advance);
                    Some(Ok(Ok(left)))
                }
            },
            (Some(value), None) => {
                try_side!(self, left, advance);
                Some(Ok(Ok(value)))
            }
            (None, Some(value)) => {
                try_side!(self, right, advance);
                Some(Ok(Ok(value)))
            }
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn merge_pure() {
        let left = vec![0u32, 7, 11, 19, 30]
            .into_iter()
            .map(Ok)
            .collect::<Vec<Result<_, ()>>>();
        let right = vec![1u32, 6, 7, 7, 8, 40]
            .into_iter()
            .map(Ok)
            .collect::<Vec<Result<_, ()>>>();

        let merged = merge(left, right)
            .collect::<Result<Result<Vec<u32>, _>, _>>()
            .unwrap()
            .unwrap();
        let expected = vec![0u32, 1, 6, 7, 8, 11, 19, 30, 40];

        assert_eq!(merged, expected);
    }

    #[test]
    fn merge_pure_bad_right() {
        let left = vec![0u32, 11, 19, 30]
            .into_iter()
            .map(Ok)
            .collect::<Vec<Result<_, ()>>>();
        let right = vec![1u32, 6, 17, 8, 40]
            .into_iter()
            .map(Ok)
            .collect::<Vec<Result<_, ()>>>();

        let merged = merge(left, right).collect::<Vec<Result<Result<u32, _>, _>>>();
        let expected = vec![
            Ok(Ok(0u32)),
            Ok(Ok(1)),
            Err(MergeError::unsorted_pair(Side::Right, 2, 17, 8)),
        ];

        assert_eq!(merged, expected);
    }

    #[test]
    fn merge_lines_readers() {
        let left = Cursor::new("apple\nbanana\ncherry\n");
        let right = Cursor::new("banana\ndate\n");

        let merged = merge_lines(left.lines(), right.lines())
            .map(|line| line.unwrap().unwrap())
            .collect::<Vec<String>>();
        let expected = vec!["apple", "banana", "cherry", "date"];

        assert_eq!(merged, expected);
    }

    #[test]
    fn merge_error_display() {
        let error = MergeError::unsorted_pair(Side::Right, 2, 17, 8);

        assert_eq!(
            error.to_string(),
            "unsorted pair in the right file at lines 3 and 4: 17, 8"
        );
    }
}
