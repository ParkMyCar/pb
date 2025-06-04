//! Single interface for registering all of the [`Config`]s for the entire build system.
//!
//! [`Config`]: pb_cfg::Config

use pb_cfg::ConfigSetBuilder;

pub fn all_cfgs(builder: &mut ConfigSetBuilder) {
    crate::register_configs(builder);
}
