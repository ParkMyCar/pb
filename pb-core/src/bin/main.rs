use anyhow::Result;
use pb_rules_core::HostStuff;
use wasmtime::*;

fn main() -> Result<()> {
    // Modules can be compiled through either the text or binary format
    let mut config = Config::new();
    config.wasm_component_model(true).wasm_multi_memory(true);
    let engine = Engine::new(&config)?;

    let mut linker = component::Linker::new(&engine);
    pb_rules_core::HostStuff::add_to_linker(
        &mut linker,
        |state: &mut pb_rules_core::HostStuff| state,
    )?;

    let pb_std =
        wasmtime::component::Component::from_file(&engine, "pb_std_rules-component.wasm").unwrap();

    for x in pb_std.component_type().exports(&engine) {
        println!("export: {x:?}");
    }

    let mut store = Store::new(&engine, HostStuff::default());
    // let std_instance = linker.instantiate(&mut store, &pb_std)?;

    let resolver = pb_rules_core::wit::RuleSet::instantiate(&mut store, &pb_std, &linker).unwrap();
    let additional_glob = resolver
        .pb_rules_resolver()
        .call_additional_interest_glob(&mut store);
    println!("{additional_glob:?}");

    let mut resource_table = wasmtime::component::ResourceTable::new();
    let file_handle = resource_table
        .push(pb_rules_core::filesystem::FileHandle::default())
        .unwrap();

    let result = resolver
        .pb_rules_resolver()
        .call_resolve_target(&mut store, file_handle);
    println!("{result:?}");

    let rule_set = resolver.pb_rules_rules().call_rule_set(&mut store).unwrap();
    for (name, _rule) in rule_set {
        println!("rule name: {name}");
    }

    // resolver.pb_rules_resolver().call_resolve_target(wasmtime::Resour, arg0)

    // let resolver = std_instance
    //     .get_export(&mut store, None, "pb:rules/resolver@0.1.0")
    //     .expect("resolver");
    // println!("{resolver:?}");
    // let additional_interest_id = std_instance
    //     .get_export(&mut store, Some(&resolver), "additional-interest-glob")
    //     .unwrap();
    // let additional_interest_func = std_instance
    //     .get_typed_func::<(), (Option<String>,)>(&mut store, additional_interest_id)
    //     .unwrap();

    // let result = additional_interest_func.call(&mut store, ());
    // println!("{result:?}");

    // let resolve_target_id = std_instance
    //     .get_export(&mut store, Some(&resolver), "resolve-target")
    //     .unwrap();
    // let resolve_target_func = std_instance
    //     .get_func(&mut store, resolve_target_id)
    //     .unwrap();
    // println!("{:?}", resolve_target_func.params(&mut store));
    // println!("{:?}", resolve_target_func.results(&mut store));

    // let resolve_target_func =
    //     std_instance
    //         .get_typed_func::<(
    //             wasmtime::component::Resource<
    //                 pb_rules_core::wit::pb::rules::read_filesystem::File,
    //             >,
    //         ), (Result<Vec<pb_rules_core::wit::pb::rules::types::Target>, String>,)>(
    //             &mut store,
    //             resolve_target_id,
    //         )
    //         .unwrap();
    // println!("{resolve_target_id:?}");

    Ok(())
}
