wit_bindgen::generate!({
    world: "target-resolver",
    path: "../pb-rules-core/wit/target",
    generate_all,
});

pub struct HttpTargetResolver;

impl exports::pb::core::resolver::Guest for HttpTargetResolver {
    fn additional_interest_glob() -> Option<_rt::String> {
        None
    }

    fn resolve_target(
        file: exports::pb::core::resolver::File,
    ) -> Result<_rt::Vec<exports::pb::core::resolver::Target>, _rt::String> {
        let location = pb::core::logging::Location {
            file_path: None,
            line: None,
        };

        let msg = format!("resolving file: {}", file.name());
        pb::core::logging::event(pb::core::logging::Level::Info, &msg, &location, &[]);

        Ok(vec![])
    }
}

export!(HttpTargetResolver);
