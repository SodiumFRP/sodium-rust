mod cell_loop_test;
mod cell_test;
mod mem_test;
mod node_test;
mod stream_test;

use crate::SodiumCtx;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn assert_memory_freed(sodium_ctx: &SodiumCtx) {
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}
