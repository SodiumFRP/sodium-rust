use crate::Operational;
use crate::SodiumCtx;
use crate::tests::assert_memory_freed;

use std::sync::Arc;
use std::sync::Mutex;

#[test]
fn loop_value_snapshot() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sodium_ctx.transaction(
                || {
                    let a = sodium_ctx.new_cell("lettuce");
                    let b = sodium_ctx.new_cell_loop();
                    let e_snap =
                        Operational
                            ::value(&a)
                            .snapshot(
                                &b.cell(),
                                |aa: &&str, bb: &&str|
                                    format!("{} {}", aa, bb)
                            );
                    b.loop_(&sodium_ctx.new_cell("cheese"));
                    e_snap.listen(move |x: &String| out.lock().as_mut().unwrap().push(x.clone()))
                }
            );
        }
        println!("{:?}", l.impl_);
        l.unlisten();
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(vec!["lettuce cheese"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_value_hold() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let value = sodium_ctx.transaction(
            || {
                let a = sodium_ctx.new_cell_loop();
                let value_ = Operational::value(&a.cell()).hold("onion");
                a.loop_(&sodium_ctx.new_cell("cheese"));
                value_
            }
        );
        let s_tick = sodium_ctx.new_stream_sink();
        let l;
        {
            let out = out.clone();
            l = s_tick.stream().snapshot1(&value).listen(
                move |x: &&'static str| out.lock().as_mut().unwrap().push(x.clone())
            );
        }
        s_tick.send(&());
        l.unlisten();
        {
            let l = out.lock();
            let out: &Vec<&'static str> = l.as_ref().unwrap();
            assert_eq!(vec!["cheese"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift_loop() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let b = sodium_ctx.new_cell_sink("kettle");
        let c = sodium_ctx.transaction(
            || {
                let a = sodium_ctx.new_cell_loop();
                let c_ = a.cell().lift2(&b.cell(), |aa: &&'static str, bb: &&'static str| format!("{} {}", aa, bb));
                a.loop_(&sodium_ctx.new_cell("tea"));
                c_
            }
        );
        let l;
        {
            let out = out.clone();
            l = c.listen(
                move |x: &String| out.lock().as_mut().unwrap().push(x.clone())
            );
        }
        b.send("caddy");
        l.unlisten();
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(vec!["tea kettle", "tea caddy"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}
