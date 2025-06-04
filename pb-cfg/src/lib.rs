//! Configuration flags for `pb` itself.
//!
//! The types in this crate should _not_ be used for configuration of build rules.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
};

use compact_str::CompactString;
use pb_ore::assert_none;

/// A single configuration setting.
pub struct Config<V: ConfigDefault> {
    name: &'static str,
    desc: &'static str,
    value: V,
}

impl<V: ConfigDefault> Config<V> {
    /// Define a new [`Config`] with a default value.
    pub const fn new(name: &'static str, desc: &'static str, default: V) -> Self {
        Config {
            name,
            desc,
            value: default,
        }
    }

    /// Read the value of this [`Config`] from the provided [`ConfigSet`].
    pub fn read(&self, set: &ConfigSet) -> V::StoredValue {
        let Some(entry) = set.configs.get(self.name) else {
            panic!("tried to read unregistered config {}", self.name);
        };
        V::from_dyn(&entry.value)
    }
}

/// A thread-safe shareable set of [`Config`]s.
#[derive(Clone, Debug)]
pub struct ConfigSet {
    configs: Arc<BTreeMap<CompactString, ConfigSetEntry>>,
}

impl ConfigSet {
    /// Returns a new [`ConfigSetBuilder`].
    pub fn builder() -> ConfigSetBuilder {
        ConfigSetBuilder::default()
    }

    /// Update [`Config`] in this [`ConfigSet`] with the specified value.
    ///
    /// # Panics
    /// * If [`Config`] was not previously registered with the original [`ConfigSetBuilder`].
    pub fn update<V: ConfigDefault>(&self, config: &'static Config<V>, value: V) {
        let entry = self
            .configs
            .get(config.name)
            .expect("tried to update unregisted config");
        entry.value.update(value.into_stored().into_dyn());
    }

    /// Update the [`Config`] in this [`ConfigSet`] with `name` to `value`.
    ///
    /// # Errors
    ///
    /// * If no config named `name` exists in this set.
    /// * If the config specified by `name` cannot parse `value`.
    ///
    pub fn try_update(&self, name: &str, value: &str) -> Result<(), anyhow::Error> {
        let entry = self
            .configs
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("not Config named '{name}' found"))?;
        entry.value.update_parse(value)?;
        Ok(())
    }
}

impl fmt::Display for ConfigSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, entry) in &*self.configs {
            writeln!(f, "{} => {}\n\t└─ '{}'", name, entry.value, entry.desc)?;
        }
        Ok(())
    }
}

/// Single entry within a [`ConfigSet`].
#[derive(Clone, Debug)]
pub struct ConfigSetEntry {
    value: DynConfigValueShared,
    desc: &'static str,
}

/// A builder for a [`ConfigSet`].
#[derive(Default, Debug)]
pub struct ConfigSetBuilder {
    configs: BTreeMap<CompactString, (DynConfigValue, &'static str)>,
}

impl ConfigSetBuilder {
    /// Register a [`Config`] into this [`ConfigSetBuilder`] with the default value.
    pub fn register<V: ConfigDefault>(&mut self, config: &'static Config<V>) -> &mut Self {
        let value = config.value.into_stored().into_dyn();
        let prev = self
            .configs
            .insert(CompactString::const_new(config.name), (value, config.desc));
        assert_none!(prev, "config '{}' registered more than once", config.name);
        self
    }

    /// Consumes this [`ConfigSetBuilder`] construting a [`ConfigSet`].
    pub fn build(self) -> ConfigSet {
        let configs = self
            .configs
            .into_iter()
            .map(|(name, (value, desc))| {
                let entry = ConfigSetEntry {
                    value: value.into_shared(),
                    desc,
                };
                (name, entry)
            })
            .collect();
        ConfigSet {
            configs: Arc::new(configs),
        }
    }
}

/// Types that can be provided as a default to a [`Config`].
pub trait ConfigDefault {
    /// The type that actually gets stored in a [`ConfigSet`].
    type StoredValue: ConfigValue;

    fn into_stored(&self) -> Self::StoredValue;
    fn from_dyn(val: &DynConfigValueShared) -> Self::StoredValue;
}

impl ConfigDefault for bool {
    type StoredValue = bool;

    fn into_stored(&self) -> Self::StoredValue {
        *self
    }

    fn from_dyn<'a>(val: &'a DynConfigValueShared) -> Self::StoredValue {
        let DynConfigValueShared::Bool(val) = val else {
            panic!("programming error, found {val:?} for string")
        };
        val.load(Ordering::SeqCst)
    }
}

impl ConfigDefault for i64 {
    type StoredValue = i64;

    fn into_stored(&self) -> Self::StoredValue {
        *self
    }

    fn from_dyn(val: &DynConfigValueShared) -> Self::StoredValue {
        let DynConfigValueShared::I64(val) = val else {
            panic!("programming error, found {val:?} for string")
        };
        val.load(Ordering::SeqCst)
    }
}

impl ConfigDefault for u64 {
    type StoredValue = u64;

    fn into_stored(&self) -> Self::StoredValue {
        *self
    }

    fn from_dyn(val: &DynConfigValueShared) -> Self::StoredValue {
        let DynConfigValueShared::U64(val) = val else {
            panic!("programming error, found {val:?} for string")
        };
        val.load(Ordering::SeqCst)
    }
}

impl ConfigDefault for &str {
    type StoredValue = CompactString;

    fn into_stored(&self) -> Self::StoredValue {
        CompactString::new(self)
    }

    fn from_dyn<'a>(val: &'a DynConfigValueShared) -> Self::StoredValue {
        let DynConfigValueShared::String(val) = val else {
            panic!("programming error, found {val:?} for string")
        };
        let read_lock = val
            .read()
            .expect("DynConfigValueShared::String lock poisoned");
        read_lock.clone()
    }
}

impl ConfigDefault for String {
    type StoredValue = CompactString;

    fn into_stored(&self) -> Self::StoredValue {
        CompactString::from(self)
    }

    fn from_dyn<'a>(val: &'a DynConfigValueShared) -> Self::StoredValue {
        let DynConfigValueShared::String(val) = val else {
            panic!("programming error, found {val:?} for string")
        };
        let read_lock = val
            .read()
            .expect("DynConfigValueShared::String lock poisoned");
        read_lock.clone()
    }
}

pub trait ConfigValue {
    fn into_dyn(self) -> DynConfigValue;
}

impl ConfigValue for bool {
    fn into_dyn(self) -> DynConfigValue {
        DynConfigValue::Bool(self)
    }
}

impl ConfigValue for i64 {
    fn into_dyn(self) -> DynConfigValue {
        DynConfigValue::I64(self)
    }
}

impl ConfigValue for u64 {
    fn into_dyn(self) -> DynConfigValue {
        DynConfigValue::U64(self)
    }
}

impl ConfigValue for CompactString {
    fn into_dyn(self) -> DynConfigValue {
        DynConfigValue::String(self)
    }
}

/// "Type erased" configuration values.
///
/// We prefer an enum as opposed to something like `Box<dyn Value>` because enums offer better
/// performance and are easier to reason about.
#[derive(Debug)]
pub enum DynConfigValue {
    Bool(bool),
    I64(i64),
    U64(u64),
    String(CompactString),
}

impl DynConfigValue {
    pub fn into_shared(self) -> DynConfigValueShared {
        match self {
            DynConfigValue::Bool(val) => DynConfigValueShared::Bool(Arc::new(AtomicBool::new(val))),
            DynConfigValue::I64(val) => DynConfigValueShared::I64(Arc::new(AtomicI64::new(val))),
            DynConfigValue::U64(val) => DynConfigValueShared::U64(Arc::new(AtomicU64::new(val))),
            DynConfigValue::String(val) => DynConfigValueShared::String(Arc::new(RwLock::new(val))),
        }
    }
}

/// Shareable instance of [`DynConfigValue`].
#[derive(Clone, Debug)]
pub enum DynConfigValueShared {
    Bool(Arc<AtomicBool>),
    I64(Arc<AtomicI64>),
    U64(Arc<AtomicU64>),
    String(Arc<RwLock<CompactString>>),
}

impl DynConfigValueShared {
    pub fn update(&self, value: DynConfigValue) {
        match (self, value) {
            (DynConfigValueShared::Bool(shared), DynConfigValue::Bool(val)) => {
                shared.store(val, Ordering::SeqCst);
            }
            (DynConfigValueShared::I64(shared), DynConfigValue::I64(val)) => {
                shared.store(val, Ordering::SeqCst);
            }
            (DynConfigValueShared::U64(shared), DynConfigValue::U64(val)) => {
                shared.store(val, Ordering::SeqCst);
            }
            (DynConfigValueShared::String(shared), DynConfigValue::String(val)) => {
                let mut write_lock = shared
                    .write()
                    .expect("DynConfigValueShared::String lock poisoned");
                *write_lock = val;
            }
            (shared, val) => unreachable!("tried to update shared {shared:?} with {val:?}"),
        }
    }

    pub fn update_parse(&self, value: &str) -> Result<(), anyhow::Error> {
        match self {
            DynConfigValueShared::Bool(shared) => {
                let val: bool = value.parse()?;
                shared.store(val, Ordering::SeqCst);
            }
            DynConfigValueShared::I64(shared) => {
                let val: i64 = value.parse()?;
                shared.store(val, Ordering::SeqCst);
            }
            DynConfigValueShared::U64(shared) => {
                let val: u64 = value.parse()?;
                shared.store(val, Ordering::SeqCst);
            }
            DynConfigValueShared::String(shared) => {
                let mut write_lock = shared
                    .write()
                    .expect("DynConfigValueShared::String lock poisoned");
                write_lock.clear();
                write_lock.push_str(value);
            }
        }

        Ok(())
    }
}

impl fmt::Display for DynConfigValueShared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynConfigValueShared::Bool(val) => {
                write!(f, "{}", val.load(Ordering::SeqCst))?;
            }
            DynConfigValueShared::I64(val) => {
                write!(f, "{}", val.load(Ordering::SeqCst))?;
            }
            DynConfigValueShared::U64(val) => {
                write!(f, "{}", val.load(Ordering::SeqCst))?;
            }
            DynConfigValueShared::String(val) => {
                let read_lock = val
                    .read()
                    .expect("DynConfigValueShared::String lock poisoned");
                write!(f, "{}", *read_lock)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub static TEST_CONFIG_A: Config<bool> =
        Config::new("test_config_a", "A test configuration value.", true);
    pub static TEST_CONFIG_B: Config<&'static str> =
        Config::new("test_config_b", "A test configuration value.", "foobar");

    #[test]
    fn smoketest_read() {
        let mut config_set = ConfigSet::builder();
        config_set.register(&TEST_CONFIG_A).register(&TEST_CONFIG_B);
        let config_set = config_set.build();

        assert_eq!(TEST_CONFIG_A.read(&config_set), true);
        assert_eq!(TEST_CONFIG_B.read(&config_set), "foobar");
    }

    #[test]
    fn smoketest_update() {
        let mut config_set = ConfigSet::builder();
        config_set.register(&TEST_CONFIG_A).register(&TEST_CONFIG_B);
        let config_set = config_set.build();
        let config_set_2 = config_set.clone();

        config_set.update(&TEST_CONFIG_A, false);
        assert_eq!(TEST_CONFIG_A.read(&config_set), false);
        assert_eq!(
            TEST_CONFIG_A.read(&config_set),
            TEST_CONFIG_A.read(&config_set_2)
        );

        config_set.update(&TEST_CONFIG_B, "hello world!");
        assert_eq!(TEST_CONFIG_B.read(&config_set), "hello world!");
        assert_eq!(
            TEST_CONFIG_B.read(&config_set),
            TEST_CONFIG_B.read(&config_set_2)
        );
    }

    #[test]
    fn smoketest_parse() {
        let mut config_set = ConfigSet::builder();
        config_set.register(&TEST_CONFIG_A).register(&TEST_CONFIG_B);
        let config_set = config_set.build();

        config_set.try_update("test_config_a", "false").unwrap();
        assert_eq!(TEST_CONFIG_A.read(&config_set), false);

        config_set
            .try_update("test_config_b", "anotha one")
            .unwrap();
        assert_eq!(TEST_CONFIG_B.read(&config_set), "anotha one");
    }
}
