use anyhow::Result;
use pb_rules_core::HostStuff;
use wasmtime::{component::types::ComponentItem, *};

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

    let pb_std = wasmtime::component::Component::from_file(&engine, "pb_std_rules-component.wasm")?;

    for x in pb_std.component_type().exports(&engine) {
        println!("export: {x:?}");
    }

    let mut store = Store::new(&engine, HostStuff::default());
    let std_instance = linker.instantiate(&mut store, &pb_std)?;

    let resolver = std_instance
        .get_export(&mut store, None, "pb:rules/resolver@0.1.0")
        .expect("resolver");
    println!("{resolver:?}");
    let additional_interest_id = std_instance
        .get_export(&mut store, Some(&resolver), "additional-interest-glob")
        .unwrap();
    let additional_interest_func = std_instance
        .get_typed_func::<(), (Option<String>,)>(&mut store, additional_interest_id)
        .unwrap();

    let result = additional_interest_func.call(&mut store, ());
    println!("{result:?}");

    Ok(())
}
