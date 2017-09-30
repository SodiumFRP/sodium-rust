use sodium::NUM_NODES;
use sodium::SodiumCtx;

pub fn assert_memory_freed(sodium_ctx: &mut SodiumCtx) {
    let num_nodes = unsafe { NUM_NODES };
    if num_nodes != 0 {
        panic!("memory leak detected, {} nodes are remaining", num_nodes);
    }
}
