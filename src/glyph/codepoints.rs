//! A collection of codepoints
//!
//! We want to preserve order and ensure uniqueness, so we use an IndexSet;
//! however we don't want this to be part of our public API, so use a wrapper.

use indexmap::IndexSet;

/// A set of Unicode codepoints
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Codepoints(IndexSet<char>);

impl Codepoints {
    /// Construct a new set of codepoints.
    ///
    ///
    /// The input can be anything that impls `IntoIterator<Item=char>`,
    /// and the simplest use would be to pass an array:
    ///
    /// ```
    /// # use norad::Codepoints;
    /// let mut codepoints = Codepoints::new(['A', 'B']);
    /// ```
    pub fn new(src: impl IntoIterator<Item = char>) -> Self {
        Self(src.into_iter().collect())
    }
    /// Return the number of codepoints.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no codepoints.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Set the codepoints. See [Codepoints::new] for usage.
    pub fn set(&mut self, codepoints: impl IntoIterator<Item = char>) {
        self.0.clear();
        self.0.extend(codepoints);
    }

    /// Remove all codepoints from the set.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Returns true if the provided codepoint is in this set.
    pub fn contains(&self, codepoint: char) -> bool {
        self.0.contains(&codepoint)
    }

    /// Insert a codepoint into the set.
    ///
    /// Returns `true` if this item did not exist in the set.
    /// If this item *does* exist, the order will be unchanged.
    pub fn insert(&mut self, codepoint: char) -> bool {
        self.0.insert(codepoint)
    }

    /// Iterate over the codepoints.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.0.iter().copied()
    }
}

impl FromIterator<char> for Codepoints {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        Codepoints(iter.into_iter().collect())
    }
}

impl IntoIterator for Codepoints {
    type Item = char;

    type IntoIter = indexmap::set::IntoIter<char>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Codepoints {
    type Item = &'a char;

    type IntoIter = indexmap::set::Iter<'a, char>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
