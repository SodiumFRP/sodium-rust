use crate::impl_::name::NodeName;
use crate::impl_::node::{IsNode, Node, IsNodeExt};
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::tests::init;

#[test]
fn node_mem_1() {
    init();
    let sodium_ctx = SodiumCtx::new();
    {
        let node1 = Node::new(&sodium_ctx, NodeName::Node(1), || {}, vec![]);
        let node2 = Node::new(&sodium_ctx, NodeName::Node(2), || {}, vec![node1.box_clone()]);
        let node3 = Node::new(&sodium_ctx, NodeName::Node(3), || {}, vec![node2.box_clone()]);
        node1.add_dependency(node3);
    }
    assert_memory_freed(&sodium_ctx);
}

#[test]
fn node_mem_2() {
    init();
    let sodium_ctx = SodiumCtx::new();
    {
        /*
             0
            / \
           3   1
            \ /
             2
            / \
           6   4
            \ /
             5
        */
        let node0 = Node::new(&sodium_ctx, NodeName::Node(0), || {}, vec![]);
        let node1 = Node::new(&sodium_ctx, NodeName::Node(1), || {}, vec![node0.box_clone()]);
        let node2 = Node::new(&sodium_ctx, NodeName::Node(2), || {}, vec![node1.box_clone()]);
        let node3 = Node::new(&sodium_ctx, NodeName::Node(3), || {}, vec![node2.box_clone()]);
        node0.add_dependency(node3);
        let node4 = Node::new(&sodium_ctx, NodeName::Node(4), || {}, vec![node2.box_clone()]);
        let node5 = Node::new(&sodium_ctx, NodeName::Node(5), || {}, vec![node4.box_clone()]);
        let node6 = Node::new(&sodium_ctx, NodeName::Node(6), || {}, vec![node5.box_clone()]);
        node2.add_dependency(node6);
    }
    assert_memory_freed(&sodium_ctx);
}

pub fn assert_memory_freed(sodium_ctx: &SodiumCtx) {
    sodium_ctx.collect_cycles();
    let node_count = sodium_ctx.node_count();
    let node_ref_count = sodium_ctx.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}
