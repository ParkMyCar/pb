use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let mut store: Store<u32> = Store::new(&engine, 4);

    // Modules can be compiled through either the text or binary format
    // let wat = r#"
    // (module
    //     (import "host" "host_func" (func $host_hello (param i32)))

    //     (func (export "hello")
    //         i32.const 3
    //         call $host_hello)
    // )
    // "#;
    // let module = Module::new(&engine, wat)?;
    let module = Module::from_file(&engine, "../target/wasm32-unknown-unknown/release/fib.wasm")?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Invoke `fib` export
    let fib = instance.get_typed_func::<i32, i32>(&mut store, "fib")?;
    println!("fib(6) = {}", fib.call(&mut store, 6)?);
    Ok(())
}
