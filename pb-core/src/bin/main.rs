use anyhow::Result;
use wasmtime::{component::types::ComponentItem, *};

fn main() -> Result<()> {
    // Modules can be compiled through either the text or binary format
    let mut config = Config::new();
    config.wasm_component_model(true).wasm_multi_memory(true);
    let engine = Engine::new(&config)?;

    let pb_core =
        wasmtime::component::Component::from_file(&engine, "pb_rules_core-component.wasm")?;
    let pb_std = wasmtime::component::Component::from_file(&engine, "pb_rules_std-component.wasm")?;

    let mut linker = component::Linker::new(&engine);

    let mut store: Store<()> = Store::new(&engine, ());
    let core_instance = linker.instantiate(&mut store, &pb_core)?;

    linker.root()
    // for (name, kind) in pb_core.component_type().exports(&engine) {
    //     match kind {
    //         ComponentItem::ComponentInstance(inst) => {
    //             inst.exports(&engine)
    //         }
    //         ComponentItem::ComponentFunc()
    //     }

    //     println!("{name}: {kind:?}");
    // }

    let std_instance = linker.instantiate(&mut store, &pb_std)?;

    // let resources = pb_std.resources_required();
    // println!("{resources:?}");

    // let component = core_instance.get_export(&mut store, None, "pb:core/logging@0.1.0");
    // println!("{component:?}");

    // let store: Store<()> = Store::new(&engine, ());

    /*
    // Create a `Linker` which will be later used to instantiate this module.
    // Host functionality is defined by name within the `Linker`.
    let mut linker = Linker::new(&engine);
    linker.func_wrap(
        "host",
        "host_func",
        |caller: Caller<'_, u32>, param: i32| {
            println!("Got {} from WebAssembly", param);
            println!("my host state is: {}", caller.data());
        },
    )?;

    // All wasm objects operate within the context of a "store". Each
    // `Store` has a type parameter to store host-specific data, which in
    // this case we're using `4` for.
    let mut store = Store::new(&engine, 4);
    let instance = linker.instantiate(&mut store, &module)?;
    let hello = instance.get_typed_func::<(), ()>(&mut store, "hello")?;

    // And finally we can call the wasm!
    hello.call(&mut store, ())?;
    */

    Ok(())
}

fn populate_linker<T>(
    engine: &Engine,
    linker: &mut component::LinkerInstance<'_, T>,
    store: &mut Store<T>,
    name: &str,
    item: ComponentItem,
) -> Result<(), anyhow::Error> {
    match item {
        ComponentItem::ComponentInstance(inst) => {
            let instance = linker.instance(name)?;
            
        }
    }

    Ok(())
}
