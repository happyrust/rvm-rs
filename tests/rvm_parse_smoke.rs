use rvm_rs::{parse_rvm, Store};

#[test]
fn parse_test_rvm_smoke() {
    let data = std::fs::read("tests/test-data/test.rvm").expect("missing test.rvm");
    let mut store = Store::new();

    parse_rvm(&data, &mut store).expect("parse_rvm should succeed");

    assert!(
        store.node_count() > 0,
        "should parse at least one node from test.rvm"
    );
}
