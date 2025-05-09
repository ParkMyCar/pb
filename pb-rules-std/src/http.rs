wit_bindgen::generate!({
    world: "target-resolver",
    generate_all,
});

pub struct HttpTargetResolver;

impl exports::pb::target::resolver::Guest for HttpTargetResolver {
    fn additional_interest_glob() -> Option<_rt::String> {
        None
    }

    fn resolve_target(
        file: exports::pb::target::resolver::File,
    ) -> Result<_rt::Vec<exports::pb::target::resolver::Target>, _rt::String> {
        Ok(vec![])
    }
}

export!(HttpTargetResolver);
