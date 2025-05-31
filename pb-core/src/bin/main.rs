use anyhow::Result;
use pb_rules_host::HostState;
use tracing_subscriber::EnvFilter;
use wasmtime::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Modules can be compiled through either the text or binary format
    let mut config = Config::new();
    config.wasm_component_model(true).wasm_multi_memory(true);
    let engine = Engine::new(&config)?;

    let mut linker = component::Linker::new(&engine);
    pb_rules_host::HostState::add_to_linker(
        &mut linker,
        |state: &mut pb_rules_host::HostState| state,
    )?;

    let pb_std =
        wasmtime::component::Component::from_file(&engine, "pb_std_rules-component.wasm").unwrap();

    for x in pb_std.component_type().exports(&engine) {
        println!("export: {x:?}");
    }

    let host_stuff = HostState::new().await;
    let mut store = Store::new(&engine, host_stuff);

    let resolver = pb_rules_host::wit::RuleSet::instantiate(&mut store, &pb_std, &linker).unwrap();
    let additional_glob = resolver
        .pb_rules_resolver()
        .call_additional_interest_glob(&mut store);
    println!("{additional_glob:?}");

    let rule_set = resolver.pb_rules_rules().call_rule_set(&mut store).unwrap();
    for (name, rule) in rule_set {
        println!("rule name: {name}, {:?}", rule.ty());

        if name == "http" {
            let context = store
                .data_mut()
                .context("http", "repository", "0.1.0", "test");
            let future = resolver
                .pb_rules_rules()
                .rule()
                .call_run(&mut store, rule, &[], context)
                .expect("failed to run rule");

            let future = futures::future::poll_fn(|cx| {
                let waker = pb_rules_host::types::HostWaker::new(cx.waker().clone());
                let waker = store.data_mut().resources.push(waker).unwrap();

                let state = resolver
                    .pb_rules_rules()
                    .rule_future()
                    .call_poll(&mut store, future, waker)
                    .expect("failed to poll");
                match state {
                    pb_rules_host::wit::exports::pb::rules::rules::RulePoll::Pending => {
                        std::task::Poll::Pending
                    }
                    pb_rules_host::wit::exports::pb::rules::rules::RulePoll::Ready(val) => {
                        std::task::Poll::Ready(val)
                    }
                }
            });

            let result = future.await;
            println!("{result:?}");
        }
    }

    Ok(())
}
