use crate::CellSink;
use crate::SodiumCtx;
use crate::StreamSink;

use crate::tests::init;

use log;

// SET RUST_LOG=trace
#[test]
fn log_test() {
    init();
    info!("a");
    log::debug!("a");
    log::debug!("b");
}

#[test]
fn mem() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream::<i32>();
        let s2 = s.map_to(5);
        let s3 = s2.map_to(3);
        let l = s3.listen_weak(|_:&i32| {});
        l.unlisten();
    }
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}

#[test]
fn map_s_mem() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let ss1: StreamSink<i32> = sodium_ctx.new_stream_sink();
        let s1 = ss1.stream();
        let _s2 = s1.map_to(5);
        let l = _s2.listen_weak(|_:&u32| {});
        println!("l: {:?}", l.impl_);
        l.unlisten();
    }
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}

#[test]
fn map_c_mem() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let cs1: CellSink<i32> = sodium_ctx.new_cell_sink(3);
        let c1 = cs1.cell();
        let _c2 = c1.map(|a:&i32| a + 5);
        //let l = _c2.listen_weak(|_:&i32| {});
        //l.unlisten();
    }
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}
