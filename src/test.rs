// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{SmartString, SmartStringMode};
use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe},
};

#[cfg(not(test))]
use arbitrary::Arbitrary;
#[cfg(test)]
use proptest::proptest;
#[cfg(test)]
use proptest_derive::Arbitrary;

pub fn assert_panic<A, F>(f: F)
where
    F: FnOnce() -> A,
{
    let old_hook = take_hook();
    set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(f));
    set_hook(old_hook);
    assert!(
        result.is_err(),
        "action that should have panicked didn't panic"
    );
}

#[derive(Arbitrary, Debug, Clone)]
pub enum Constructor {
    New,
    FromString(String),
    FromStringSlice(String),
}

impl Constructor {
    pub fn construct<Mode: SmartStringMode>(self) -> (String, SmartString<Mode>) {
        match self {
            Self::New => (String::new(), SmartString::new()),
            Self::FromString(string) => (string.clone(), SmartString::from(string)),
            Self::FromStringSlice(string) => (string.clone(), SmartString::from(string.as_str())),
        }
    }
}

#[derive(Arbitrary, Debug, Clone)]
pub enum TestBounds {
    Range(usize, usize),
    From(usize),
    To(usize),
    Full,
    Inclusive(usize, usize),
    ToInclusive(usize),
}

impl TestBounds {
    fn should_panic(&self, control: &str) -> bool {
        let len = control.len();
        match self {
            Self::Range(start, end)
                if start > end
                    || start > &len
                    || end > &len
                    || !control.is_char_boundary(*start)
                    || !control.is_char_boundary(*end) =>
            {
                true
            }
            Self::From(start) if start > &len || !control.is_char_boundary(*start) => true,
            Self::To(end) if end > &len || !control.is_char_boundary(*end) => true,
            Self::Inclusive(start, end)
                if start > end
                    || start > &len
                    || end > &len
                    || !control.is_char_boundary(*start)
                    || !control.is_char_boundary(*end + 1) =>
            {
                true
            }
            Self::ToInclusive(end) if end > &len || !control.is_char_boundary(*end + 1) => true,
            _ => false,
        }
    }

    fn assert_range<A, B>(&self, control: &A, subject: &B)
    where
        A: Index<Range<usize>>,
        B: Index<Range<usize>>,
        A: Index<RangeFrom<usize>>,
        B: Index<RangeFrom<usize>>,
        A: Index<RangeTo<usize>>,
        B: Index<RangeTo<usize>>,
        A: Index<RangeFull>,
        B: Index<RangeFull>,
        A: Index<RangeInclusive<usize>>,
        B: Index<RangeInclusive<usize>>,
        A: Index<RangeToInclusive<usize>>,
        B: Index<RangeToInclusive<usize>>,
        <A as Index<Range<usize>>>::Output: PartialEq<<B as Index<Range<usize>>>::Output> + Debug,
        <B as Index<Range<usize>>>::Output: Debug,
        <A as Index<RangeFrom<usize>>>::Output:
            PartialEq<<B as Index<RangeFrom<usize>>>::Output> + Debug,
        <B as Index<RangeFrom<usize>>>::Output: Debug,
        <A as Index<RangeTo<usize>>>::Output:
            PartialEq<<B as Index<RangeTo<usize>>>::Output> + Debug,
        <B as Index<RangeTo<usize>>>::Output: Debug,
        <A as Index<RangeFull>>::Output: PartialEq<<B as Index<RangeFull>>::Output> + Debug,
        <B as Index<RangeFull>>::Output: Debug,
        <A as Index<RangeInclusive<usize>>>::Output:
            PartialEq<<B as Index<RangeInclusive<usize>>>::Output> + Debug,
        <B as Index<RangeInclusive<usize>>>::Output: Debug,
        <A as Index<RangeToInclusive<usize>>>::Output:
            PartialEq<<B as Index<RangeToInclusive<usize>>>::Output> + Debug,
        <B as Index<RangeToInclusive<usize>>>::Output: Debug,
    {
        match self {
            Self::Range(start, end) => assert_eq!(control[*start..*end], subject[*start..*end]),
            Self::From(start) => assert_eq!(control[*start..], subject[*start..]),
            Self::To(end) => assert_eq!(control[..*end], subject[..*end]),
            Self::Full => assert_eq!(control[..], subject[..]),
            Self::Inclusive(start, end) => {
                assert_eq!(control[*start..=*end], subject[*start..=*end])
            }
            Self::ToInclusive(end) => assert_eq!(control[..=*end], subject[..=*end]),
        }
    }
}

#[derive(Arbitrary, Debug, Clone)]
pub enum Action {
    Slice(TestBounds),
    Push(char),
    PushStr(String),
    Truncate(usize),
    Pop,
    Remove(usize),
    Insert(usize, char),
    InsertStr(usize, String),
    SplitOff(usize),
    Clear,
    IntoString,
    Retain(String),
    // Drain(TestBounds),
    // ReplaceRange(TestBounds, String),
}

impl Action {
    pub fn perform<Mode: SmartStringMode>(
        self,
        control: &mut String,
        subject: &mut SmartString<Mode>,
    ) {
        match self {
            Self::Slice(range) => {
                if range.should_panic(&control) {
                    assert_panic(|| range.assert_range(control, subject))
                } else {
                    range.assert_range(control, subject);
                }
            }
            Self::Push(ch) => {
                control.push(ch);
                subject.push(ch);
            }
            Self::PushStr(ref string) => {
                control.push_str(string);
                subject.push_str(string);
            }
            Self::Truncate(index) => {
                if index <= control.len() && !control.is_char_boundary(index) {
                    assert_panic(|| control.truncate(index));
                    assert_panic(|| subject.truncate(index));
                } else {
                    control.truncate(index);
                    subject.truncate(index);
                }
            }
            Self::Pop => {
                assert_eq!(control.pop(), subject.pop());
            }
            Self::Remove(index) => {
                if index >= control.len() || !control.is_char_boundary(index) {
                    assert_panic(|| control.remove(index));
                    assert_panic(|| subject.remove(index));
                } else {
                    assert_eq!(control.remove(index), subject.remove(index));
                }
            }
            Self::Insert(index, ch) => {
                if index > control.len() || !control.is_char_boundary(index) {
                    assert_panic(|| control.insert(index, ch));
                    assert_panic(|| subject.insert(index, ch));
                } else {
                    control.insert(index, ch);
                    subject.insert(index, ch);
                }
            }
            Self::InsertStr(index, ref string) => {
                if index > control.len() || !control.is_char_boundary(index) {
                    assert_panic(|| control.insert_str(index, string));
                    assert_panic(|| subject.insert_str(index, string));
                } else {
                    control.insert_str(index, string);
                    subject.insert_str(index, string);
                }
            }
            Self::SplitOff(index) => {
                if !control.is_char_boundary(index) {
                    assert_panic(|| control.split_off(index));
                    assert_panic(|| subject.split_off(index));
                } else {
                    assert_eq!(control.split_off(index), subject.split_off(index));
                }
            }
            Self::Clear => {
                control.clear();
                subject.clear();
            }
            Self::IntoString => {
                assert_eq!(control, &Into::<String>::into(subject.clone()));
            }
            Self::Retain(filter) => {
                let f = |ch| filter.contains(ch);
                control.retain(f);
                subject.retain(f);
            } // FIXME: skipping `drain` and `replace_range` tests, pending https://github.com/rust-lang/rust/issues/72237
              // Self::Drain(range) => {
              //     if range.should_panic(&control) {
              //         assert_panic(|| match range {
              //             TestBounds::Range(start, end) => {
              //                 (control.drain(start..end), subject.drain(start..end))
              //             }
              //             TestBounds::From(start) => (control.drain(start..), subject.drain(start..)),
              //             TestBounds::To(end) => (control.drain(..end), subject.drain(..end)),
              //             TestBounds::Full => (control.drain(..), subject.drain(..)),
              //             TestBounds::Inclusive(start, end) => {
              //                 (control.drain(start..=end), subject.drain(start..=end))
              //             }
              //             TestBounds::ToInclusive(end) => {
              //                 (control.drain(..=end), subject.drain(..=end))
              //             }
              //         })
              //     } else {
              //         let (control_iter, subject_iter) = match range {
              //             TestBounds::Range(start, end) => {
              //                 (control.drain(start..end), subject.drain(start..end))
              //             }
              //             TestBounds::From(start) => (control.drain(start..), subject.drain(start..)),
              //             TestBounds::To(end) => (control.drain(..end), subject.drain(..end)),
              //             TestBounds::Full => (control.drain(..), subject.drain(..)),
              //             TestBounds::Inclusive(start, end) => {
              //                 (control.drain(start..=end), subject.drain(start..=end))
              //             }
              //             TestBounds::ToInclusive(end) => {
              //                 (control.drain(..=end), subject.drain(..=end))
              //             }
              //         };
              //         let control_result: String = control_iter.collect();
              //         let subject_result: String = subject_iter.collect();
              //         assert_eq!(control_result, subject_result);
              //     }
              // }
              // Self::ReplaceRange(range, string) => {
              //     if range.should_panic(&control) {
              //         assert_panic(|| match range {
              //             TestBounds::Range(start, end) => {
              //                 control.replace_range(start..end, &string);
              //                 subject.replace_range(start..end, &string);
              //             }
              //             TestBounds::From(start) => {
              //                 control.replace_range(start.., &string);
              //                 subject.replace_range(start.., &string);
              //             }
              //             TestBounds::To(end) => {
              //                 control.replace_range(..end, &string);
              //                 subject.replace_range(..end, &string);
              //             }
              //             TestBounds::Full => {
              //                 control.replace_range(.., &string);
              //                 subject.replace_range(.., &string);
              //             }
              //             TestBounds::Inclusive(start, end) => {
              //                 control.replace_range(start..=end, &string);
              //                 subject.replace_range(start..=end, &string);
              //             }
              //             TestBounds::ToInclusive(end) => {
              //                 control.replace_range(..=end, &string);
              //                 subject.replace_range(..=end, &string);
              //             }
              //         })
              //     } else {
              //         match range {
              //             TestBounds::Range(start, end) => {
              //                 control.replace_range(start..end, &string);
              //                 subject.replace_range(start..end, &string);
              //             }
              //             TestBounds::From(start) => {
              //                 control.replace_range(start.., &string);
              //                 subject.replace_range(start.., &string);
              //             }
              //             TestBounds::To(end) => {
              //                 control.replace_range(..end, &string);
              //                 subject.replace_range(..end, &string);
              //             }
              //             TestBounds::Full => {
              //                 control.replace_range(.., &string);
              //                 subject.replace_range(.., &string);
              //             }
              //             TestBounds::Inclusive(start, end) => {
              //                 control.replace_range(start..=end, &string);
              //                 subject.replace_range(start..=end, &string);
              //             }
              //             TestBounds::ToInclusive(end) => {
              //                 control.replace_range(..=end, &string);
              //                 subject.replace_range(..=end, &string);
              //             }
              //         }
              //     }
              // }
        }
    }
}

fn assert_invariants<Mode: SmartStringMode>(control: &str, subject: &SmartString<Mode>) {
    assert_eq!(control, subject.as_str());
    assert_eq!(control.len(), subject.len());
    assert_eq!(
        subject.is_inline(),
        subject.len() <= Mode::MAX_INLINE,
        "len {} should be inline (MAX_INLINE = {}) but was boxed",
        subject.len(),
        Mode::MAX_INLINE
    );
    assert_eq!(
        control.partial_cmp(&"ordering test".to_string()),
        subject.partial_cmp("ordering test")
    );
    let control_smart: SmartString<Mode> = control.into();
    assert_eq!(Ordering::Equal, subject.cmp(&control_smart));
}

pub fn test_everything<Mode: SmartStringMode>(constructor: Constructor, actions: Vec<Action>) {
    let (mut control, mut subject): (_, SmartString<Mode>) = constructor.construct();
    assert_invariants(&control, &subject);
    for action in actions {
        action.perform(&mut control, &mut subject);
        assert_invariants(&control, &subject);
    }
}

pub fn test_ordering<Mode: SmartStringMode>(left: String, right: String) {
    let smart_left = SmartString::<Mode>::from(&left);
    let smart_right = SmartString::<Mode>::from(&right);
    assert_eq!(left.cmp(&right), smart_left.cmp(&smart_right));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Compact, Prefixed};

    proptest! {
        #[test]
        fn proptest_everything_compact(constructor: Constructor, actions: Vec<Action>) {
            test_everything::<Compact>(constructor, actions);
        }

        #[test]
        fn proptest_everything_prefixed(constructor: Constructor, actions: Vec<Action>) {
            test_everything::<Prefixed>(constructor, actions);
        }

        #[test]
        fn proptest_ordering_compact(left: String, right: String) {
            test_ordering::<Compact>(left,right)
        }

        #[test]
        fn proptest_ordering_prefixed(left: String, right: String) {
            test_ordering::<Prefixed>(left,right)
        }
    }

    #[test]
    fn must_panic_on_insert_outside_char_boundary() {
        test_everything::<Prefixed>(
            Constructor::FromString("a0 A୦a\u{2de0}0 🌀Aa".to_string()),
            vec![
                Action::Push(' '),
                Action::Push('¡'),
                Action::Pop,
                Action::Pop,
                Action::Push('¡'),
                Action::Pop,
                Action::Push('𐀀'),
                Action::Push('\u{e000}'),
                Action::Pop,
                Action::Insert(14, 'A'),
            ],
        );
    }

    #[test]
    fn must_panic_on_out_of_bounds_range() {
        test_everything::<Prefixed>(
            Constructor::New,
            vec![Action::Slice(TestBounds::Range(0, 13764126361151078400))],
        );
    }

    #[test]
    fn must_not_promote_before_insert_succeeds() {
        test_everything::<Prefixed>(
            Constructor::FromString("ኲΣ A𑒀a ®Σ a0🠀  aA®A".to_string()),
            vec![Action::Insert(21, ' ')],
        );
    }

    #[test]
    fn must_panic_on_slice_outside_char_boundary() {
        test_everything::<Prefixed>(
            Constructor::New,
            vec![Action::Push('Ь'), Action::Slice(TestBounds::ToInclusive(0))],
        )
    }

    #[test]
    fn must_compare_correctly_with_different_fragment_char_counts() {
        test_ordering::<Prefixed>("\u{1b}\u{7be}\nJ\\#\u{7be}J\\\no\u{7be}\n\n[\n\u{2}\n\n\u{11}C\u{0}\u{0}\u{0}A\n\n[\n\u{2}\n\n\u{11}C\u{0}A\n\u{1a}\n\u{7be}JC\u{11}\u{10}C\u{0}[\u{2}\u{1b}\u{7be}\nJ\\XX".to_string(),
            "\u{1b}\u{7be}\nJ\\\u{7be}\n\n[\n\u{2}\n\n\u{11}C\u{0}\u{0}\u{0}A\n\n[\n\u{2}\n\n\u{11}C\u{0}A\n\u{1a}\n\u{7be}JC\u{11}\u{10}C\u{0}[\u{2}\u{1b}\u{7be}\nJ\\XX\u{1b}\u{7be}\nJ\\#\u{7be}J\\\no\u{7be}\n\n[\n\u{2}\n\n\u{11}C\u{0}\u{0}\u{0}A\n\n[\n\u{2}\n\n\u{11}C\u{0}A\n\u{1a}\n\u{7be}JC\u{11}\u{10}C\u{0}[\u{2}\u{1b}\u{7be}\nJ\\XXXXXXXXX\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}\u{1f}XXXXXXXXXXXXXXXXXXXXXXXXX".to_string());
    }

    #[test]
    fn dont_panic_when_inserting_a_string_at_exactly_inline_capacity() {
        let string: String = (0..Compact::MAX_INLINE).map(|_| '\u{0}').collect();
        test_everything::<Compact>(Constructor::New, vec![Action::InsertStr(0, string)])
    }

    // #[test]
    // fn drain_a_string_properly() {
    //     test_everything::<Compact>(
    //         Constructor::New,
    //         vec![Action::Push('¡'), Action::Drain(TestBounds::Full)],
    //     )
    // }

    #[test]
    #[should_panic]
    fn drain_bounds_integer_overflow_must_panic() {
        // test_everything::<Compact>(
        //     Constructor::FromString("מ∢∢∢∢∢∢∢∢".to_string()),
        //     vec![Action::Drain(TestBounds::ToInclusive(usize::max_value()))],
        // )
        let mut string = SmartString::<Compact>::from("מ");
        string.drain(..=usize::max_value());
    }

    #[test]
    fn string_drain_overflow() {
        let mut string = String::new();
        string.drain(..=usize::max_value());
    }
}
