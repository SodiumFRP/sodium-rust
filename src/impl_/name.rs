use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug)]
pub enum NodeName {
    Node(u8),
    Cell(Cell),
    Stream(Stream),
    ListenerNew,
    Router,
    NullNode,
}

impl NodeName {
    pub const CELL_NEW: NodeName = NodeName::Cell(Cell::New);
    pub const CELL_HOLD: NodeName = NodeName::Cell(Cell::Hold);
    pub const CELL_SWITCH_S_INNER: NodeName = NodeName::Cell(Cell::SwitchSInner);
    pub const CELL_SWITCH_S_OUTER: NodeName = NodeName::Cell(Cell::SwitchSOuter);
    pub const CELL_SWITCH_C_INNER: NodeName = NodeName::Cell(Cell::SwitchCInner);
    pub const CELL_SWITCH_C_OUTER: NodeName = NodeName::Cell(Cell::SwitchCOuter);

    pub const STREAM_NEW: NodeName = NodeName::Stream(Stream::New);
    pub const STREAM_NEW_WITH_COALESCER: NodeName = NodeName::Stream(Stream::NewWithCoalescer);
    pub const STREAM_MAP: NodeName = NodeName::Stream(Stream::Map);
    pub const STREAM_FILTER: NodeName = NodeName::Stream(Stream::Filter);
    pub const STREAM_MERGE: NodeName = NodeName::Stream(Stream::Merge);
    pub const STREAM_ONCE: NodeName = NodeName::Stream(Stream::Once);
    pub const STREAM_LISTEN: NodeName = NodeName::Stream(Stream::Listen);
    pub const STREAM_LOOP_NEW: NodeName = NodeName::Stream(Stream::LoopNew);
}

#[derive(Clone, Debug)]
pub enum Cell {
    New,
    Hold,
    SwitchSInner,
    SwitchSOuter,
    SwitchCInner,
    SwitchCOuter,

}

#[derive(Clone, Debug)]
pub enum Stream {
    New,
    NewWithCoalescer,
    Map,
    Filter,
    Merge,
    Once,
    Listen,
    LoopNew,
}

impl Display for NodeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
            NodeName::Node(n) => write!(f, "node{}", n),

            NodeName::Cell(Cell::New)          => f.write_str("Cell::new"),
            NodeName::Cell(Cell::Hold)         => f.write_str("Cell::hold"),
            NodeName::Cell(Cell::SwitchSInner) => f.write_str("switch_s inner node"),
            NodeName::Cell(Cell::SwitchSOuter) => f.write_str("switch_s outer node"),
            NodeName::Cell(Cell::SwitchCInner) => f.write_str("switch_c inner node"),
            NodeName::Cell(Cell::SwitchCOuter) => f.write_str("switch_c outer node"),

            NodeName::Stream(Stream::New)              => f.write_str("Stream::new"),
            NodeName::Stream(Stream::NewWithCoalescer) => f.write_str("Stream::_new_with_coalescer"),
            NodeName::Stream(Stream::Map)              => f.write_str("Stream::map"),
            NodeName::Stream(Stream::Filter)           => f.write_str("Stream::filter"),
            NodeName::Stream(Stream::Merge)            => f.write_str("Stream::merge"),
            NodeName::Stream(Stream::Once)             => f.write_str("Stream::once"),
            NodeName::Stream(Stream::Listen)           => f.write_str("Stream::listen"),
            NodeName::Stream(Stream::LoopNew)          => f.write_str("StreamLoop::new"),

            NodeName::ListenerNew => f.write_str("Listener::new"),
            NodeName::Router      => f.write_str("Router"),
            NodeName::NullNode    => f.write_str("null_node"),
        }
    }
}
