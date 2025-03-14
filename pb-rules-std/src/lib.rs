//! "Standard Library" rules for `pb`.

use pb_rules_core::RuleSet;
use serde::{Deserialize, Serialize};

pub struct HttpRules;

impl RuleSet for HttpRules {
    fn run(path: Vec<String>, attrs: rmpv::Value) -> impl Iterator<Item = rmpv::Value> {
        std::iter::once(rmpv::Value::String("todo".into()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpArchive {
    url: String,
}
