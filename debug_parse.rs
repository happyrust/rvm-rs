use rvm_rs::{parse_rvm, Store};

fn main() {
    let data = std::fs::read("tests/test-data/test.rvm").expect("missing test.rvm");
    let mut store = Store::new();

    match parse_rvm(&data, &mut store) {
        Ok(_) => println!("Parse succeeded! Node count: {}", store.node_count()),
        Err(e) => println!("Parse failed: {:?}", e),
    }
}
