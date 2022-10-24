//! This module implements the functionality described in
//! ["Strictly Pretty" (2000) by Christian Lindig][0], with a few
//! extensions.
//!
//! This module is heavily influenced by Elixir's Inspect.Algebra and
//! JavaScript's Prettier.
//!
//! [0]: http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.34.2200
//!
//! ## Extensions
//!
//! - `ForceBreak` from Prettier.
//! - `FlexBreak` from Elixir.
#![allow(clippy::wrong_self_convention)]

// #[cfg(test)]
// mod tests;

use std::collections::VecDeque;

use itertools::Itertools;

#[macro_export]
macro_rules! docvec {
    () => {
        Document::Vec(Vec::new())
    };

    ($($x:expr),+ $(,)?) => {
        Document::Vec(vec![$($x.to_doc()),+])
    };
}

#[derive(Debug)]
pub enum Error {}

/// Coerce a value into a Document.
/// Note we do not implement this for String as a slight pressure to favour str
/// over String.
pub trait Documentable<'a> {
    fn to_doc(self) -> Document<'a>;
}

impl<'a> Documentable<'a> for char {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for &'a str {
    fn to_doc(self) -> Document<'a> {
        Document::Str(self)
    }
}

impl<'a> Documentable<'a> for isize {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for i64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for usize {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for f64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{:?}", self))
    }
}

impl<'a> Documentable<'a> for u64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{:?}", self))
    }
}

impl<'a> Documentable<'a> for u32 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for u16 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for u8 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{}", self))
    }
}

impl<'a> Documentable<'a> for Document<'a> {
    fn to_doc(self) -> Document<'a> {
        self
    }
}

impl<'a> Documentable<'a> for Vec<Document<'a>> {
    fn to_doc(self) -> Document<'a> {
        Document::Vec(self)
    }
}

impl<'a, D: Documentable<'a>> Documentable<'a> for Option<D> {
    fn to_doc(self) -> Document<'a> {
        self.map(Documentable::to_doc).unwrap_or_else(nil)
    }
}

pub fn concat<'a>(docs: impl IntoIterator<Item = Document<'a>>) -> Document<'a> {
    Document::Vec(docs.into_iter().collect())
}

pub fn join<'a>(
    docs: impl IntoIterator<Item = Document<'a>>,
    separator: Document<'a>,
) -> Document<'a> {
    concat(Itertools::intersperse(docs.into_iter(), separator))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document<'a> {
    /// A mandatory linebreak
    Line(usize),

    /// Forces contained groups to break
    ForceBreak,

    /// May break contained document based on best fit, thus flex break
    FlexBreak(Box<Self>),

    /// Renders `broken` if group is broken, `unbroken` otherwise
    Break {
        broken: &'a str,
        unbroken: &'a str,
        kind: BreakKind,
    },

    /// Join multiple documents together
    Vec(Vec<Self>),

    /// Nests the given document by the given indent
    Nest(isize, Box<Self>),

    /// Nests the given document to the current cursor position
    NestCurrent(Box<Self>),

    /// Nests the given document to the current cursor position
    Group(Box<Self>),

    /// A string to render
    String(String),

    /// A str to render
    Str(&'a str),
}

#[derive(Debug, Clone)]
enum Mode {
    Broken,
    Unbroken,
}

fn fits(
    mut limit: isize,
    mut current_width: isize,
    mut docs: VecDeque<(isize, Mode, &Document<'_>)>,
) -> bool {
    loop {
        if current_width > limit {
            return false;
        };

        let (indent, mode, document) = match docs.pop_front() {
            Some(x) => x,
            None => return true,
        };

        match document {
            Document::Line(_) => return true,

            Document::ForceBreak => return false,

            Document::Nest(i, doc) => docs.push_front((i + indent, mode, doc)),

            // TODO: Remove
            Document::NestCurrent(doc) => docs.push_front((indent, mode, doc)),

            Document::Group(doc) => docs.push_front((indent, Mode::Unbroken, doc)),

            Document::Str(s) => limit -= s.len() as isize,
            Document::String(s) => limit -= s.len() as isize,

            Document::Break { unbroken, .. } => match mode {
                Mode::Broken => return true,
                Mode::Unbroken => current_width += unbroken.len() as isize,
            },

            Document::FlexBreak(doc) => docs.push_front((indent, mode, doc)),

            Document::Vec(vec) => {
                for doc in vec.iter().rev() {
                    docs.push_front((indent, mode.clone(), doc));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakKind {
    Flex,
    Strict,
}

fn format(
    writer: &mut String,
    limit: isize,
    mut width: isize,
    mut docs: VecDeque<(isize, Mode, &Document<'_>)>,
) -> Result<(), Error> {
    while let Some((indent, mode, document)) = docs.pop_front() {
        match document {
            Document::ForceBreak => (),

            Document::Line(i) => {
                for _ in 0..*i {
                    writer.push('\n');
                }

                for _ in 0..indent {
                    writer.push(' ');
                }

                width = indent;
            }

            // Flex breaks are NOT conditional to the mode
            Document::Break {
                broken,
                unbroken,
                kind: BreakKind::Flex,
            } => {
                let unbroken_width = width + unbroken.len() as isize;

                if fits(limit, unbroken_width, docs.clone()) {
                    writer.push_str(unbroken);

                    width = unbroken_width;
                } else {
                    writer.push_str(broken);

                    writer.push('\n');

                    for _ in 0..indent {
                        writer.push(' ');
                    }
                    width = indent;
                }
            }

            // Strict breaks are conditional to the mode
            Document::Break {
                broken,
                unbroken,
                kind: BreakKind::Strict,
            } => {
                width = match mode {
                    Mode::Unbroken => {
                        writer.push_str(unbroken);

                        width + unbroken.len() as isize
                    }
                    Mode::Broken => {
                        writer.push_str(broken);

                        writer.push('\n');

                        for _ in 0..indent {
                            writer.push(' ');
                        }

                        indent
                    }
                };
            }

            Document::String(s) => {
                width += s.len() as isize;

                writer.push_str(s);
            }

            Document::Str(s) => {
                width += s.len() as isize;

                writer.push_str(s);
            }

            Document::Vec(vec) => {
                for doc in vec.iter().rev() {
                    docs.push_front((indent, mode.clone(), doc));
                }
            }

            Document::Nest(i, doc) => {
                docs.push_front((indent + i, mode, doc));
            }

            Document::NestCurrent(doc) => {
                docs.push_front((width, mode, doc));
            }

            Document::Group(doc) | Document::FlexBreak(doc) => {
                // TODO: don't clone the doc
                let mut group_docs = VecDeque::new();

                group_docs.push_back((indent, Mode::Unbroken, doc.as_ref()));

                if fits(limit, width, group_docs) {
                    docs.push_front((indent, Mode::Unbroken, doc));
                } else {
                    docs.push_front((indent, Mode::Broken, doc));
                }
            }
        }
    }
    Ok(())
}

pub fn nil<'a>() -> Document<'a> {
    Document::Vec(vec![])
}

pub fn line<'a>() -> Document<'a> {
    Document::Line(1)
}

pub fn lines<'a>(i: usize) -> Document<'a> {
    Document::Line(i)
}

pub fn force_break<'a>() -> Document<'a> {
    Document::ForceBreak
}

pub fn break_<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Strict,
    }
}

pub fn flex_break<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Flex,
    }
}

impl<'a> Document<'a> {
    pub fn group(self) -> Self {
        Self::Group(Box::new(self))
    }

    pub fn nest(self, indent: isize) -> Self {
        Self::Nest(indent, Box::new(self))
    }

    pub fn nest_current(self) -> Self {
        Self::NestCurrent(Box::new(self))
    }

    pub fn append(self, second: impl Documentable<'a>) -> Self {
        match self {
            Self::Vec(mut vec) => {
                vec.push(second.to_doc());
                Self::Vec(vec)
            }
            first => Self::Vec(vec![first, second.to_doc()]),
        }
    }

    pub fn to_pretty_string(self, limit: isize) -> String {
        let mut buffer = String::new();

        self.pretty_print(limit, &mut buffer)
            .expect("Writing to string buffer failed");

        buffer
    }

    pub fn surround(self, open: impl Documentable<'a>, closed: impl Documentable<'a>) -> Self {
        open.to_doc().append(self).append(closed)
    }

    pub fn pretty_print(&self, limit: isize, writer: &mut String) -> Result<(), Error> {
        let mut docs = VecDeque::new();

        docs.push_back((0, Mode::Unbroken, self));

        format(writer, limit, 0, docs)?;

        Ok(())
    }

    /// Returns true when the document contains no printable characters
    /// (whitespace and newlines are considered printable characters).
    pub fn is_empty(&self) -> bool {
        use Document::*;
        match self {
            Line(n) => *n == 0,
            ForceBreak => true,
            String(s) => s.is_empty(),
            Str(s) => s.is_empty(),
            // assuming `broken` and `unbroken` are equivalent
            Break { broken, .. } => broken.is_empty(),
            FlexBreak(d) | Nest(_, d) | NestCurrent(d) | Group(d) => d.is_empty(),
            Vec(docs) => docs.iter().all(|d| d.is_empty()),
        }
    }
}