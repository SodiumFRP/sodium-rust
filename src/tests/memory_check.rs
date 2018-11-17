use sodium::SodiumCtx;

pub fn assert_memory_freed(sodium_ctx: &mut SodiumCtx) {
    let node_count = sodium_ctx.node_count();
    if node_count != 0 {
        panic!("memory leak detected, {} nodes are remaining", node_count);
    }
}
