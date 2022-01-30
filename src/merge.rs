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

#[derive(Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if *self == Side::Left { "left" } else { "right" })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MergeError<T> {
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

impl<T: Debug> Display for MergeError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsortedPair {
                side,
                index,
                first,
                second,
            } => {
                write!(
                    f,
                    "unsorted pair in the {} file at lines {} and {}: {:?}, {:?}",
                    side,
                    index + 1,
                    index + 2,
                    first,
                    second
                )
            }
        }
    }
}

impl<T: Debug> std::error::Error for MergeError<T> {}

pub struct Merge<T, IL, IR> {
    iter_l: Option<IL>,
    iter_r: Option<IR>,
    index_l: usize,
    index_r: usize,
    last_l: Option<T>,
    last_r: Option<T>,
    pending_l: Option<T>,
    pending_r: Option<T>,
    started: bool,
}

type NestedResult<T, E, A> = Result<Result<A, E>, MergeError<T>>;

impl<T: Ord, E, IL: Iterator<Item = Result<T, E>>, IR: Iterator<Item = Result<T, E>>>
    Merge<T, IL, IR>
{
    fn new(left: IL, right: IR) -> Self {
        Self {
            iter_l: Some(left),
            iter_r: Some(right),
            index_l: 0,
            index_r: 0,
            last_l: None,
            last_r: None,
            pending_l: None,
            pending_r: None,
            started: false,
        }
    }

    fn read_l(&mut self) -> Result<Option<T>, E> {
        match self.iter_l.as_mut().and_then(|iter| iter.next().take()) {
            Some(Ok(value)) => {
                self.index_l += 1;
                Ok(Some(value))
            }
            Some(Err(error)) => {
                self.iter_l = None;
                self.iter_r = None;
                Err(error)
            }
            None => {
                self.iter_l = None;
                Ok(None)
            }
        }
    }

    fn read_r(&mut self) -> Result<Option<T>, E> {
        match self.iter_r.as_mut().and_then(|iter| iter.next().take()) {
            Some(Ok(value)) => {
                self.index_r += 1;
                Ok(Some(value))
            }
            Some(Err(error)) => {
                self.iter_l = None;
                self.iter_r = None;
                Err(error)
            }
            None => {
                self.iter_r = None;
                Ok(None)
            }
        }
    }

    fn read_next_l(&mut self, last: T) -> NestedResult<T, E, (T, Option<T>)> {
        loop {
            match self.read_l() {
                Ok(Some(value)) => match last.cmp(&value) {
                    Ordering::Less => {
                        return Ok(Ok((last, Some(value))));
                    }
                    Ordering::Equal => {}
                    Ordering::Greater => {
                        return Err(MergeError::unsorted_pair(
                            Side::Left,
                            self.index_l - 2,
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

    fn read_next_r(&mut self, last: T) -> NestedResult<T, E, (T, Option<T>)> {
        loop {
            match self.read_r() {
                Ok(Some(value)) => match last.cmp(&value) {
                    Ordering::Less => {
                        return Ok(Ok((last, Some(value))));
                    }
                    Ordering::Equal => {}
                    Ordering::Greater => {
                        return Err(MergeError::unsorted_pair(
                            Side::Right,
                            self.index_r - 2,
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

    fn advance_l(&mut self) -> Result<Result<(), E>, MergeError<T>> {
        if let Some(pending) = self.pending_l.take() {
            match self.read_next_l(pending) {
                Ok(Ok((new_last, new_pending))) => {
                    self.last_l = Some(new_last);
                    self.pending_l = new_pending;
                    Ok(Ok(()))
                }
                Ok(Err(error)) => Ok(Err(error)),
                Err(error) => Err(error),
            }
        } else {
            self.last_l = None;
            Ok(Ok(()))
        }
    }

    fn advance_r(&mut self) -> Result<Result<(), E>, MergeError<T>> {
        if let Some(pending) = self.pending_r.take() {
            match self.read_next_r(pending) {
                Ok(Ok((new_last, new_pending))) => {
                    self.last_r = Some(new_last);
                    self.pending_r = new_pending;
                    Ok(Ok(()))
                }
                Ok(Err(error)) => Ok(Err(error)),
                Err(error) => Err(error),
            }
        } else {
            self.last_r = None;
            Ok(Ok(()))
        }
    }
}

impl<T: Ord, E, IL: Iterator<Item = Result<T, E>>, IR: Iterator<Item = Result<T, E>>> Iterator
    for Merge<T, IL, IR>
{
    type Item = Result<Result<T, E>, MergeError<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.started = true;

            match self.read_l() {
                Ok(Some(value)) => match self.read_next_l(value) {
                    Ok(Ok((last, pending))) => {
                        self.last_l = Some(last);
                        self.pending_l = pending;
                    }
                    Ok(Err(error)) => {
                        return Some(Ok(Err(error)));
                    }
                    Err(error) => {
                        return Some(Err(error));
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    return Some(Ok(Err(error)));
                }
            }

            match self.read_r() {
                Ok(Some(value)) => match self.read_next_r(value) {
                    Ok(Ok((last, pending))) => {
                        self.last_r = Some(last);
                        self.pending_r = pending;
                    }
                    Ok(Err(error)) => {
                        return Some(Ok(Err(error)));
                    }
                    Err(error) => {
                        return Some(Err(error));
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    return Some(Ok(Err(error)));
                }
            }
        }

        match (self.last_l.take(), self.last_r.take()) {
            (Some(last_l), Some(last_r)) => match last_l.cmp(&last_r) {
                Ordering::Less => {
                    match self.advance_l() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            return Some(Ok(Err(error)));
                        }
                        Err(error) => {
                            return Some(Err(error));
                        }
                    }
                    self.last_r = Some(last_r);
                    Some(Ok(Ok(last_l)))
                }
                Ordering::Greater => {
                    match self.advance_r() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            return Some(Ok(Err(error)));
                        }
                        Err(error) => {
                            return Some(Err(error));
                        }
                    }
                    self.last_l = Some(last_l);
                    Some(Ok(Ok(last_r)))
                }
                Ordering::Equal => {
                    match self.advance_l() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            return Some(Ok(Err(error)));
                        }
                        Err(error) => {
                            return Some(Err(error));
                        }
                    }
                    match self.advance_r() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            return Some(Ok(Err(error)));
                        }
                        Err(error) => {
                            return Some(Err(error));
                        }
                    }
                    Some(Ok(Ok(last_l)))
                }
            },
            (Some(last), None) => {
                match self.advance_l() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        return Some(Ok(Err(error)));
                    }
                    Err(error) => {
                        return Some(Err(error));
                    }
                }

                Some(Ok(Ok(last)))
            }
            (None, Some(last)) => {
                match self.advance_r() {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        return Some(Ok(Err(error)));
                    }
                    Err(error) => {
                        return Some(Err(error));
                    }
                }

                Some(Ok(Ok(last)))
            }
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
