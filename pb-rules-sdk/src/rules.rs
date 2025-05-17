//! Rust traits for defining a `pb` build rule.

use std::borrow::Cow;
use std::collections::BTreeMap;

use futures::future::BoxFuture;

use crate::futures::GuestFutureAdapter;

/// The attributes or fields provided to a rule.
#[derive(Debug, Clone)]
pub struct Attributes {
    pub inner: BTreeMap<String, crate::pb::rules::types::Attribute>,
}

pub trait RuleSet {
    /// Return the set of rules provided by this rule set.
    fn rule_set() -> Vec<(String, Box<dyn Rule>)>;
}

impl<S: RuleSet> crate::exports::pb::rules::rules::Guest for S {
    type Rule = Box<dyn Rule>;
    type RuleFuture = GuestFutureAdapter<Vec<crate::pb::rules::types::Provider>>;

    fn rule_set() -> crate::_rt::Vec<(crate::_rt::String, crate::exports::pb::rules::rules::Rule)> {
        let rules = <S as RuleSet>::rule_set();
        rules
            .into_iter()
            .map(|(name, rule)| {
                let rule = crate::exports::pb::rules::rules::Rule::new(rule);
                (name, rule)
            })
            .collect()
    }
}

pub trait Rule: Send + Sync + 'static {
    /// The name of this rule.
    fn name(&self) -> Cow<'static, str>;

    /// Run the build rule.
    fn execute(
        &self,
        attrs: Attributes,
        context: crate::pb::rules::context::Ctx,
    ) -> BoxFuture<'static, Vec<crate::pb::rules::types::Provider>>;
}

impl<R: Rule + 'static> crate::exports::pb::rules::rules::GuestRule for R {
    fn name() -> crate::_rt::String {
        "TODO".to_string()
        // crate::logging::with_logging(|| <R as Rule>::name().to_string())
    }

    fn run(
        &self,
        attrs: crate::_rt::Vec<(
            crate::_rt::String,
            crate::exports::pb::rules::rules::Attribute,
        )>,
        context: crate::exports::pb::rules::rules::Ctx,
    ) -> crate::exports::pb::rules::rules::RuleFuture {
        let attrs = Attributes {
            inner: attrs.into_iter().collect(),
        };
        let fut = <R as Rule>::execute(&self, attrs, context);
        let adapter = GuestFutureAdapter::new(fut);
        crate::exports::pb::rules::rules::RuleFuture::new(adapter)
    }
}

impl crate::exports::pb::rules::rules::GuestRule for Box<dyn Rule> {
    fn name() -> crate::_rt::String {
        "TODO".to_string()
        // crate::logging::with_logging(|| <R as Rule>::name().to_string())
    }

    fn run(
        &self,
        attrs: crate::_rt::Vec<(
            crate::_rt::String,
            crate::exports::pb::rules::rules::Attribute,
        )>,
        context: crate::exports::pb::rules::rules::Ctx,
    ) -> crate::exports::pb::rules::rules::RuleFuture {
        let attrs = Attributes {
            inner: attrs.into_iter().collect(),
        };
        let fut = self.execute(attrs, context);
        let adapter = GuestFutureAdapter::new(fut);
        crate::exports::pb::rules::rules::RuleFuture::new(adapter)
    }
}

impl crate::exports::pb::rules::rules::GuestRuleFuture
    for GuestFutureAdapter<Vec<crate::pb::rules::types::Provider>>
{
    fn poll(
        &self,
        waker: crate::exports::pb::rules::rules::Waker,
    ) -> crate::exports::pb::rules::rules::RulePoll {
        match crate::logging::with_logging(|| self.poll(waker)) {
            std::task::Poll::Ready(result) => {
                crate::exports::pb::rules::rules::RulePoll::Ready(result)
            }
            std::task::Poll::Pending => crate::exports::pb::rules::rules::RulePoll::Pending,
        }
    }
}
