//! Rust traits for defining a `pb` resolver.

use crate::exports;

pub trait Resolver {
    fn additional_interest_glob() -> Option<String>;
    fn resolve_target(
        file: Vec<u8>,
    ) -> Result<Vec<crate::exports::pb::rules::resolver::Target>, String>;
}

impl<R: Resolver + 'static> exports::pb::rules::resolver::Guest for R {
    fn additional_interest_glob() -> Option<crate::_rt::String> {
        crate::logging::with_logging(|| <R as Resolver>::additional_interest_glob())
    }

    fn resolve_target(
        file: exports::pb::rules::resolver::File,
    ) -> Result<crate::_rt::Vec<exports::pb::rules::resolver::Target>, crate::_rt::String> {
        crate::logging::with_logging(|| {
            let contents = vec![];
            <R as Resolver>::resolve_target(contents)
        })
    }
}
