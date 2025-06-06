//! Iterator utilities.

/// A trait for dealing with iterators that can borrow from themselves.
///
/// Very similar to [`std::iter::Iterator`] except the associated [`LendingIterator::Item`] type
/// is generic over a lifetime. This allows implementations of [`LendingIterator`] to return
/// references to types that it owns.
///
/// # Example
///
/// ```ignore
/// struct FileReader {
///   handle: Handle,
///   block: Vec<u8>,
/// }
///
/// impl LendingIterator for FileReader {
///     type Item<'a> = &'a [u8] where Self: 'a;
///
///     fn next(&mut self) -> Option<Self::Item<'_>> {
///         // On every iterator we re-use the same block of memory instead of
///         // allocating a new one.
///         handle.read(&mut self.block[..]);
///         &self.block[..]
///     }
/// }
/// ```
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}
