use sodium::SodiumCtx;

pub fn assert_memory_freed(sodium_ctx: &mut SodiumCtx) {
    let num_nodes = (*sodium_ctx.data).borrow().num_nodes;
    if num_nodes != 0 {
        panic!("memory leak detected, {} nodes are remaining", num_nodes);
    }
}
