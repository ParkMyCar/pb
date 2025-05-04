use anyhow::Result;
use wasmtime::{component::types::ComponentItem, *};

#[tokio::main]
async fn main() -> Result<()> {
    // Modules can be compiled through either the text or binary format
    let engine = Engine::default();

    let pb_core =
        wasmtime::component::Component::from_file(&engine, "pb_rules_core-component.wasm")?;
    for (name, x) in pb_core.component_type().imports(&engine) {
        println!("core import {name}: {x:?}");
    }
    for (name, x) in pb_core.component_type().exports(&engine) {
        match &x {
            ComponentItem::ComponentInstance(inst) => {
                for (name, y) in inst.exports(&engine) {
                    if let ComponentItem::ComponentFunc(f) = &y {
                        let params: Vec<_> = f.params().collect();
                        println!("{params:?}");
                    }
                    println!("core export nested {name}: {y:?}");
                }
            }
            _ => panic!("foobar"),
        }
        println!("core export {name}: {x:?}");
    }

    let pb_std = wasmtime::component::Component::from_file(&engine, "pb_rules_std-component.wasm")?;
    for (name, x) in pb_std.component_type().imports(&engine) {
        match &x {
            ComponentItem::ComponentInstance(inst) => {
                for (name, y) in inst.exports(&engine) {
                    if let ComponentItem::ComponentFunc(f) = &y {
                        let params: Vec<_> = f.params().collect();
                        println!("{params:?}");
                    }
                    println!("std import nested {name}: {y:?}");
                }
            }
            _ => panic!("foobar"),
        }
        println!("std import {name}: {x:?}");
    }
    for (name, x) in pb_std.component_type().exports(&engine) {
        println!("std export {name}: {x:?}");
    }

    let linker = component::Linker::new(&engine);

    let mut store: Store<()> = Store::new(&engine, ());
    let core_instance = linker.instantiate_async(&mut store, &pb_core).await?;

    // let component = core_instance.get_export(&mut store, None, "pb:core/logging@0.1.0");
    // println!("{component:?}");

    let store: Store<()> = Store::new(&engine, ());
    let std_instance = linker.instantiate(store, &pb_std)?;

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
