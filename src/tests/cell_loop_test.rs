use sodium::Cell;
use sodium::CellLoop;
use sodium::CellSink;
use sodium::IsCell;
use sodium::IsStream;
use sodium::Operational;
use sodium::SodiumCtx;
use sodium::StreamSink;
use sodium::Transaction;
use tests::assert_memory_freed;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn loop_value_snapshot() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Transaction::run(
                sodium_ctx,
                |sodium_ctx| {
                    let a = Cell::new(sodium_ctx, "lettuce");
                    let b = CellLoop::new(sodium_ctx);
                    let e_snap =
                        Operational
                            ::value(sodium_ctx, &a)
                            .snapshot(
                                sodium_ctx,
                                &b,
                                |aa: &&str, bb: &&str|
                                    format!("{} {}", aa, bb)
                            );
                    let mut sodium_ctx2 = sodium_ctx.clone();
                    let sodium_ctx2 = &mut sodium_ctx2;
                    b.loop_(sodium_ctx, &Cell::new(sodium_ctx2, "cheese"));
                    e_snap.listen(sodium_ctx, move |x| out.borrow_mut().push(x.clone()))
                }
            );
        }
        l.unlisten();
        assert_eq!(vec!["lettuce cheese"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_value_hold() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Rc::new(RefCell::new(Vec::new()));
        let value = Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                let a = CellLoop::new(sodium_ctx);
                let value_ = Operational::value(sodium_ctx, &a).hold(sodium_ctx, "onion");
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                a.loop_(sodium_ctx, &Cell::new(sodium_ctx2, "cheese"));
                value_
            }
        );
        let s_tick = StreamSink::new(sodium_ctx);
        let l;
        {
            let out = out.clone();
            l = s_tick.snapshot_to(sodium_ctx, &value).listen(
                sodium_ctx,
                move |x| out.borrow_mut().push(x.clone())
            );
        }
        s_tick.send(sodium_ctx, &());
        l.unlisten();
        assert_eq!(vec!["cheese"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift_loop() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Rc::new(RefCell::new(Vec::new()));
        let b = CellSink::new(sodium_ctx, "kettle");
        let c = Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                let a = CellLoop::new(sodium_ctx);
                let c_ = a.lift2(sodium_ctx, &b, |aa, bb| format!("{} {}", aa, bb));
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                a.loop_(sodium_ctx, &Cell::new(sodium_ctx2, "tea"));
                c_
            }
        );
        let l;
        {
            let out = out.clone();
            l = c.listen(
                sodium_ctx,
                move |x| out.borrow_mut().push(x.clone())
            );
        }
        b.send(sodium_ctx, &"caddy");
        l.unlisten();
        assert_eq!(vec!["tea kettle", "tea caddy"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}
