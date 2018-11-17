use sodium::CellLoop;
use sodium::CellSink;
use sodium::IsCell;
use sodium::IsStream;
use sodium::Operational;
use sodium::SodiumCtx;
use sodium::StreamSink;
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
            l = sodium_ctx.transaction(
                |sodium_ctx| {
                    let a = sodium_ctx.new_cell("lettuce");
                    let mut b = sodium_ctx.new_cell_loop();
                    let e_snap =
                        Operational
                            ::value(&a)
                            .snapshot2(
                                &b,
                                |aa: &&str, bb: &&str|
                                    format!("{} {}", aa, bb)
                            );
                    let mut sodium_ctx2 = sodium_ctx.clone();
                    let sodium_ctx2 = &mut sodium_ctx2;
                    b.loop_(&sodium_ctx2.new_cell("cheese"));
                    e_snap.listen(move |x| out.borrow_mut().push(x.clone()))
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
        let value = sodium_ctx.transaction(
            |sodium_ctx| {
                let mut a = sodium_ctx.new_cell_loop();
                let value_ = Operational::value(&a).hold("onion");
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                a.loop_(&sodium_ctx2.new_cell("cheese"));
                value_
            }
        );
        let s_tick = sodium_ctx.new_stream_sink();
        let l;
        {
            let out = out.clone();
            l = s_tick.snapshot(&value).listen(
                move |x| out.borrow_mut().push(x.clone())
            );
        }
        s_tick.send(&());
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
        let b = sodium_ctx.new_cell_sink("kettle");
        let c = sodium_ctx.transaction(
            |sodium_ctx| {
                let mut a = sodium_ctx.new_cell_loop();
                let c_ = a.lift2(&b, |aa: &&'static str, bb: &&'static str| format!("{} {}", aa, bb));
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                a.loop_(&sodium_ctx2.new_cell("tea"));
                c_
            }
        );
        let l;
        {
            let out = out.clone();
            l = c.listen(
                move |x| out.borrow_mut().push(x.clone())
            );
        }
        b.send(&"caddy");
        l.unlisten();
        assert_eq!(vec!["tea kettle", "tea caddy"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}
