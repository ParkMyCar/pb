pub trait TargetResolver {
    /// Return a glob pattern that expresses what additional files, other than
    /// pb.toml, that this rule set is interested in for target resolution.
    fn additional_interest_glob() -> Option<String>;

    /// Given the contents of a file, return all of the targets present.
    fn resolve_targets(file: &[u8]) -> impl Iterator<Item = rmpv::Value>;
}

pub trait RuleSet {
    fn run(path: Vec<String>, attrs: rmpv::Value) -> impl Iterator<Item = rmpv::Value>;
}
