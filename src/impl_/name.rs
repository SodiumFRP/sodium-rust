#[derive(Clone, Debug)]
pub enum NodeName {
    Node(u8),
    Cell(Cell),
    Stream(Stream),
    ListenerNew,
    Router,
    NullNode,
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

pub enum Listener {
    New,
}
