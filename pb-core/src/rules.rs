//! Build rules.

use pb_rules_host::{wit::exports::pb::rules::rules::Attribute, HostState};
use wasmtime::Store;

use crate::defs::RuleSpec;

pub static REQUIRED_STD_RULES: &[&str] = &["http-repository"];

pub struct StdRules {
    /// Pre-instantiated rule set.
    rule_set_pre: pb_rules_host::wit::RuleSetPre<HostState>,
    /// The underlying WASM component.
    component: wasmtime::component::Component,
}

impl StdRules {
    pub fn try_load(
        spec: &RuleSpec,
        linker: &wasmtime::component::Linker<HostState>,
        engine: &wasmtime::Engine,
        host_state: &HostState,
    ) -> Result<StdRules, anyhow::Error> {
        let (rule_set_pre, component) = match spec {
            RuleSpec::Local { path } => {
                let component = wasmtime::component::Component::from_file(engine, path)?;
                let instance_pre = linker.instantiate_pre(&component)?;
                let rule_set = pb_rules_host::wit::RuleSetPre::new(instance_pre)?;
                (rule_set, component)
            }
            RuleSpec::Remote { .. } => {
                anyhow::bail!("remote spec is not supported for 'std' rules");
            }
            RuleSpec::Version(_) => {
                anyhow::bail!("'std' rules not yet bundled with binary");
            }
        };

        let mut store = Store::new(&engine, host_state.clone());
        let std_rules = rule_set_pre.instantiate(&mut store)?;

        let rule_set = std_rules.pb_rules_rules().call_rule_set(&mut store)?;
        let names: Vec<_> = rule_set.into_iter().map(|(name, _)| name).collect();
        tracing::debug!(?names, "loaded std rules");

        Ok(StdRules {
            rule_set_pre,
            component,
        })
    }

    pub async fn http_repository(
        &self,
        engine: &wasmtime::Engine,
        host_state: &HostState,
        name: String,
        url: String,
    ) -> Result<(), anyhow::Error> {
        let mut store = Store::new(engine, host_state.clone());
        let std_rules = self.rule_set_pre.instantiate(&mut store)?;

        let context = store
            .data_mut()
            .context("std", "http-repository", "0.1.0", "test");
        let attributes = vec![
            ("name".to_string(), Attribute::Text(name)),
            ("url".to_string(), Attribute::Text(url)),
        ];

        let rules = std_rules.pb_rules_rules().call_rule_set(&mut store)?;
        let (_name, http_repository) = rules
            .into_iter()
            .find(|(name, _rule)| name == "http-repository")
            .expect("http-repository should exist");
        let future = std_rules.pb_rules_rules().rule().call_run(
            &mut store,
            http_repository,
            &attributes[..],
            context,
        )?;

        let result = futures::future::poll_fn(|cx| {
            let waker = pb_rules_host::types::HostWaker::new(cx.waker().clone());
            let waker = store.data_mut().resources.push(waker).unwrap();

            let state = std_rules
                .pb_rules_rules()
                .rule_future()
                .call_poll(&mut store, future, waker)
                .expect("failed to poll");
            match state {
                pb_rules_host::wit::exports::pb::rules::rules::RulePoll::Pending => {
                    std::task::Poll::Pending
                }
                pb_rules_host::wit::exports::pb::rules::rules::RulePoll::Ready(val) => {
                    std::task::Poll::Ready(val)
                }
            }
        })
        .await;
        tracing::info!(?result, "ran rule!");

        Ok(())
    }
}

/// A set of build rules that have been loaded and ready to instantiate.
pub struct LoadedRuleSet {
    /// Pre-instantiated rule set.
    rule_set_pre: pb_rules_host::wit::RuleSetPre<HostState>,
    /// The underlying WASM component.
    component: wasmtime::component::Component,
}

impl LoadedRuleSet {
    pub fn try_load(
        spec: &RuleSpec,
        linker: &wasmtime::component::Linker<HostState>,
        engine: &wasmtime::Engine,
    ) -> Result<LoadedRuleSet, anyhow::Error> {
        let (rule_set_pre, component) = match spec {
            RuleSpec::Local { path } => {
                let component = wasmtime::component::Component::from_file(engine, path)?;
                let instance_pre = linker.instantiate_pre(&component)?;
                let rule_set = pb_rules_host::wit::RuleSetPre::new(instance_pre)?;
                (rule_set, component)
            }
            RuleSpec::Remote { .. } => {
                anyhow::bail!("remote spec is not supported for 'std' rules");
            }
            RuleSpec::Version(_) => {
                anyhow::bail!("'std' rules not yet bundled with binary");
            }
        };

        Ok(LoadedRuleSet {
            rule_set_pre,
            component,
        })
    }
}
