//! Rust traits for defining a `pb` resolver.

use crate::exports;

pub trait Resolver {
    type Iterator: TargetDiffIterator;

    fn new() -> Self;

    fn additional_interest_glob() -> Option<String>;

    fn process_update(&self, update: exports::pb::rules::target_resolver::ManifestUpdate);

    fn target_diffs(&self) -> Self::Iterator;
}

impl<R: Resolver + 'static> exports::pb::rules::target_resolver::GuestResolver for R {
    fn new() -> Self {
        todo!()
    }

    fn additional_interest_glob() -> Option<crate::_rt::String> {
        todo!()
    }

    fn process_update(&self, update: exports::pb::rules::target_resolver::ManifestUpdate) -> () {}

    fn target_diffs(&self) -> exports::pb::rules::target_resolver::TargetDiffIterator {
        todo!()
    }
}

pub trait TargetDiffIterator {
    fn next(&self) -> Option<exports::pb::rules::target_resolver::ResolvedTarget>;
}

impl<T: TargetDiffIterator + 'static> exports::pb::rules::target_resolver::GuestTargetDiffIterator
    for T
{
    fn next(&self) -> Option<exports::pb::rules::target_resolver::ResolvedTarget> {
        todo!()
    }
}

impl<R: Resolver + 'static> exports::pb::rules::target_resolver::Guest for R {
    type Resolver = R;
    type TargetDiffIterator = R::Iterator;
}
