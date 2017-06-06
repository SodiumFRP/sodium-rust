extern crate topological_sort;

pub mod rusty_frp;

#[cfg(test)]
mod tests {
    use rusty_frp::Cell;
    use rusty_frp::CellSink;
    use rusty_frp::IsCell;
    use rusty_frp::FrpContext;
    use rusty_frp::Stream;
    use rusty_frp::StreamSink;
    use rusty_frp::IsStream;
    use rusty_frp::WithFrpContext;
    /*
    use rusty_frp::Cell;
    use rusty_frp::CellSink;
    use rusty_frp::CellTrait;
    use rusty_frp::FrpContext;
    use rusty_frp::Stream;
    use rusty_frp::StreamSink;
    use rusty_frp::StreamTrait;
    use rusty_frp::WithFrpContext;

    /*
    #[test]
    fn test_via_console() {
        // use: cargo test -- --nocapture
        test_cell_sink();
        test_cell_map();
        test_stream_map();
        test_lift2();
        test_cell_loop();
    }
    */

    fn test_cell_sink() {
        println!("test_cell_sink");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs1 = env.frp_context.new_cell_sink(1u32);
        cs1.observe(&mut env, &with_frp_context, |_, value| { println!("cs1 = {}", value); });
        cs1.change_value(&mut env, &with_frp_context, 2);
        cs1.change_value(&mut env, &with_frp_context, 3);
        cs1.change_value(&mut env, &with_frp_context, 4);
    }

    fn test_cell_map() {
        println!("test_cell_map");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs1 = env.frp_context.new_cell_sink(1u32);
        let c2 = env.frp_context.map_c(&cs1, |value| { value + 1 });
        c2.observe(&mut env, &with_frp_context, |_, value| { println!("c2 = {}", value); });
        cs1.change_value(&mut env, &with_frp_context, 2);
        cs1.change_value(&mut env, &with_frp_context, 3);
        cs1.change_value(&mut env, &with_frp_context, 4);
    }

    fn test_stream_map() {
        println!("test_stream_map");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let ss1 = env.frp_context.new_stream_sink();
        let s2 = env.frp_context.map_s(&ss1, |value| { value + 1 });
        s2.observe(&mut env, &with_frp_context, |_, value| { println!("c2 = {}", value); });
        ss1.send(&mut env, &with_frp_context, 2);
        ss1.send(&mut env, &with_frp_context, 3);
        ss1.send(&mut env, &with_frp_context, 4);
    }

    fn test_lift2() {
        println!("test_lift2");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs1 = env.frp_context.new_cell_sink(1u32);
        let cs2 = env.frp_context.new_cell_sink(1u32);
        let c3 =
            env.frp_context.lift2_c(
                |a, b| a + b,
                &cs1,
                &cs2
            );
        c3.observe(&mut env, &with_frp_context, |_, value| { println!("c3 = {}", value); });
        cs1.change_value(&mut env, &with_frp_context, 2);
        cs2.change_value(&mut env, &with_frp_context, 3);
        cs1.change_value(&mut env, &with_frp_context, 4);
    }

    fn test_cell_loop() {
        println!("test_cell_loop");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs_pulse: CellSink<Env,Option<()>> = env.frp_context.new_cell_sink(None);
        let c_pulse = cs_pulse.clone();
        let c: Cell<Env,u32> =
            env.frp_context.loop_c(
                0u32,
                move |frp_context, c| {
                    frp_context.lift2_c(
                        |a: &u32, pulse| {
                            match pulse {
                                &Some(_) => a.clone() + 1,
                                &None => a.clone()
                            }
                        },
                        c,
                        &c_pulse
                    )
                }
            );
        c.observe(&mut env, &with_frp_context, |_,value| { println!("c = {}", value); });
        // pulse 1
        cs_pulse.change_value(&mut env, &with_frp_context, Some(()));
        // pulse 2
        cs_pulse.change_value(&mut env, &with_frp_context, Some(()));
        // pulse 3
        cs_pulse.change_value(&mut env, &with_frp_context, Some(()));
    }
/*
function shouldThrow(substr : string, f : () => void) : void {
    try {
        f();
    }
    catch (err) {
        if (err.message.search(substr) >= 0)
            return;
        else
            fail("unexpected exception: "+err);
    }
    fail("should throw exception");
}

let current_test : string = null;

function checkMemory() : void {
    if (getTotalRegistrations() != 0)
        throw new Error("listeners were not deregistered!");
}

function test(name : string, t : () => void)
{
    current_test = name;
    let pass = true;
    try {
        t();
        checkMemory();
        current_test = null
        console.log(name + " - PASS");
    }
    catch (err) {
        console.log(name + " - FAIL:");
        if (err.stack !== undefined)
            console.log(err.stack);
        else
            console.log(err);
        pass = false;
        current_test = null;
    }
}*/

    #[test]
    fn map() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s2 = env.frp_context.map_s(&s, |a| a + 1);
        s2.observe(
            &mut env,
            &with_frp_context,
            |env:&mut Env, value:&u32|
                env.out.push(value.clone())
        );
        s.send(&mut env, &with_frp_context, 7);
        assert_eq!(env.out, vec![8]);
    }

/*
test("send_with_no_listener_1", () => {
    shouldThrow("invoked before listeners",
        () => {
            const s = new StreamSink<number>();
            s.send(7);
        }
    );
});

test("send_with_no_listener_2", () => {
    () => {
        const s = new StreamSink<number>();
        const out : number[] = [];
        const kill = s.map(a => a + 1)
                    .listen(a => out.push(a));
        s.send(7);
        kill();
        s.send(9);  // this should not throw, because once() uses this mechanism
    }
});

test("map_track", () => {
    const s = new StreamSink<number>(),
        t = new StreamSink<string>(),
        out : number[] = [],
        kill = s.map(lambda1((a : number) => a + 1, [t]))
                .listen(a => out.push(a));
    s.send(7);
    t.send("banana");
    kill();
    assertEquals([8], out);
});

test("mapTo", () => {
    const s = new StreamSink<number>(),
        out : string[] = [],
        kill = s.mapTo("fusebox")
                .listen(a => out.push(a));
    s.send(7);
    s.send(9);
    kill();
    assertEquals(["fusebox", "fusebox"], out);
});
*/

    #[test]
    fn merge_non_simultaneous() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s1: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s2: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s3 = env.frp_context.or_else(&s2, &s1);
        s3.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        s1.send(&mut env, &with_frp_context, 7);
        s2.send(&mut env, &with_frp_context, 9);
        s1.send(&mut env, &with_frp_context, 8);
        assert_eq!(vec!(7,9,8), env.out);
    }

    #[test]
    fn merge_simultaneous() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s1: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s2: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s3 = env.frp_context.or_else(&s2, &s1);
        s3.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                s1.send(env, with_frp_context, 7);
                s2.send(env, with_frp_context, 60);
            }
        );
        FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                s1.send(env, with_frp_context, 9);
            }
        );
        FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                s1.send(env, with_frp_context, 7);
                s1.send(env, with_frp_context, 60);
                s2.send(env, with_frp_context, 8);
                s2.send(env, with_frp_context, 90);
            }
        );
        FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                s2.send(env, with_frp_context, 8);
                s2.send(env, with_frp_context, 90);
                s1.send(env, with_frp_context, 7);
                s1.send(env, with_frp_context, 60);
            }
        );
        FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                s2.send(env, with_frp_context, 8);
                s1.send(env, with_frp_context, 7);
                s2.send(env, with_frp_context, 90);
                s1.send(env, with_frp_context, 60);
            }
        );
        assert_eq!(vec![60,9,90,90,90], env.out);
    }
/*
test("coalesce", () => {
    const s = new StreamSink<number>((a, b) => a+b),
        out : number[] = [],
        kill = s.listen(a => out.push(a));
    Transaction.run<void>(() => {
        s.send(2);
    });
    Transaction.run<void>(() => {
        s.send(8);
        s.send(40);
    });
    kill();
    assertEquals([2, 48], out);
});
*/

    #[test]
    fn filter() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s2 = env.frp_context.filter(|a| a.clone() < 10, &s);
        s2.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        s.send(&mut env, &with_frp_context, 2);
        s.send(&mut env, &with_frp_context, 16);
        s.send(&mut env, &with_frp_context, 9);
        assert_eq!(vec![2, 9], env.out);
    }

    #[test]
    fn merge2() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let sa: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let sb_1 = env.frp_context.map_s(&sa, |x| x.clone() / 10);
        let sb = env.frp_context.filter(|x| x.clone() != 0, &sb_1);
        let sc_1 = env.frp_context.map_s(&sa, |x| x.clone() % 10);
        let sc = env.frp_context.merge(&sc_1, &sb, |x, y| x.clone() + y.clone());
        sc.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        sa.send(&mut env, &with_frp_context, 2);
        sa.send(&mut env, &with_frp_context, 52);
        assert_eq!(vec![2, 7], env.out);
    }

/*
test("loopStream", () => {
    const sa = new StreamSink<number>(),
        sc = Transaction.run(() => {
            const sb = new StreamLoop<number>(),
                sc_ = sa.map(x => x % 10).merge(sb,
                    (x, y) => x+y),
                sb_out = sa.map(x => Math.floor(x / 10))
                           .filter(x => x != 0);
            sb.loop(sb_out);
            return sc_;
        }),
        out : number[] = [],
        kill = sc.listen(a => out.push(a));
    sa.send(2);
    sa.send(52);
    kill();
    assertEquals([2, 7], out);
});
*/

    #[test]
    fn gate() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s: StreamSink<Env,String> = env.frp_context.new_stream_sink();
        let pred = env.frp_context.new_cell_sink(true);
        let s2 = env.frp_context.gate(&s, &pred);
        s2.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        s.send(&mut env, &with_frp_context, String::from("H"));
        pred.change_value(&mut env, &with_frp_context, false);
        s.send(&mut env, &with_frp_context, String::from("O"));
        pred.change_value(&mut env, &with_frp_context, true);
        s.send(&mut env, &with_frp_context, String::from("I"));
        assert_eq!(vec![String::from("H"), String::from("I")], env.out);
    }

/*
test("collect", () => {
    const ea = new StreamSink<number>(),
        out : number[] = [],
        sum = ea.collect(0, (a, s) => new Tuple2(a+s+100, a+s)),
        kill = sum.listen(a => out.push(a));
    ea.send(5);
    ea.send(7);
    ea.send(1);
    ea.send(2);
    ea.send(3);
    kill();
    assertEquals([105,112,113,115,118], out);
});

test("accum", () => {
    const ea = new StreamSink<number>(),
        out : number[] = [],
        sum = ea.accum(100, (a, s) => a + s),
        kill = sum.listen(a => out.push(a));
    ea.send(5);
    ea.send(7);
    ea.send(1);
    ea.send(2);
    ea.send(3);
    kill();
    assertEquals([100,105,112,113,115,118], out);
});

test("once", () => {

    const s = new StreamSink<string>(),
        out : string[] = [],
        kill = s.once().listen(a => out.push(a));
    s.send("A");
    s.send("B");
    s.send("C");
    kill();
    assertEquals(["A"], out);
});

test("defer", () => {
    const s = new StreamSink<string>(),
        c = s.hold(" "),
        out : string[] = [],
        kill = Operational.defer(s).snapshot1(c)
               .listen(a => out.push(a));
    s.send("C");
    s.send("B");
    s.send("A");
    kill();
    assertEquals(["C","B","A"], out);
});
*/

    #[test]
    fn hold() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let c = env.frp_context.hold(0, &s);
        let s2 = env.frp_context.updates(&c);
        s2.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        s.send(&mut env, &with_frp_context, 2);
        s.send(&mut env, &with_frp_context, 9);
        assert_eq!(vec![2, 9], env.out);
    }

    #[test]
    fn snapshot() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let c: CellSink<Env,u32> = env.frp_context.new_cell_sink(0);
        let s: StreamSink<Env,u32> = env.frp_context.new_stream_sink();
        let s2 = env.frp_context.snapshot(|x, y| format!("{} {}", x, y), &s, &c);
        s2.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        s.send(&mut env, &with_frp_context, 100);
        c.change_value(&mut env, &with_frp_context, 2);
        s.send(&mut env, &with_frp_context, 200);
        c.change_value(&mut env, &with_frp_context, 9);
        c.change_value(&mut env, &with_frp_context, 1);
        s.send(&mut env, &with_frp_context, 300);
        assert_eq!(vec![String::from("100 0"), String::from("200 2"), String::from("300 1")], env.out);
    }

    #[test]
    fn values() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let c: CellSink<Env,u32> = env.frp_context.new_cell_sink(9);
        c.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        c.change_value(&mut env, &with_frp_context, 2);
        c.change_value(&mut env, &with_frp_context, 7);
        assert_eq!(vec![9, 2, 7], env.out);
    }

    #[test]
    fn constant_cell() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let c: Cell<Env,u32> = env.frp_context.constant(12);
        c.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        assert_eq!(vec![12], env.out);
    }
*/
    #[test]
    fn map_c() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let c: CellSink<Env,u32> = env.frp_context.new_cell_sink(6);
        let c2 = c.map(&mut env.frp_context, |a| format!("{}", a));
        c2.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        c.send(&mut env, &with_frp_context, 8);
        assert_eq!(vec![String::from("6"), String::from("8")], env.out);
    }
/*
/*
test("mapCLateListen", () => {
    shouldThrow("invoked before listeners", () => {
        const c = new CellSink<number>(6),
            out : string[] = [],
            cm = c.map(a => ""+a);
        c.send(2);
        const kill = cm.listen(a => out.push(a));
        c.send(8);
        kill();
        assertEquals(["2", "8"], out);
    });
});
*/
*/
    #[test]
    fn apply() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cf: CellSink<Env,Box<Fn(&u32)->String>> = env.frp_context.new_cell_sink(Box::new(|a| format!("1 {}", a)));
        let ca: CellSink<Env,u32> = env.frp_context.new_cell_sink(5);
        let c = ca.apply(&mut env.frp_context, &cf);
        c.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        cf.send(&mut env, &with_frp_context, Box::new(|a| format!("12 {}", a)));
        ca.send(&mut env, &with_frp_context, 6);
        assert_eq!(vec![String::from("1 5"), String::from("12 5"), String::from("12 6")], env.out);
    }

    #[test]
    fn lift() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let a = env.frp_context.new_cell_sink(1);
        let b = env.frp_context.new_cell_sink(5);
        let c = a.lift2(&mut env.frp_context, &b, |aa, bb| format!("{} {}", aa, bb));
        c.observe(&mut env, &with_frp_context, |env,value| env.out.push(value.clone()));
        a.send(&mut env, &with_frp_context, 12);
        b.send(&mut env, &with_frp_context, 6);
        assert_eq!(vec![String::from("1 5"), String::from("12 5"), String::from("12 6")], env.out);
    }

    /*
    #[test]
    fn lift_glitch() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let a = env.frp_context.new_cell_sink(1);
        let a3 = env.frp_context.map_c(&a, |x| x.clone() * 3);
        let a5 = env.frp_context.map_c(&a, |x| x.clone() * 5);
        let b = env.frp_context.lift2_c(|x, y| format!("{} {}", x, y), &a3, &a5);
        b.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        a.change_value(&mut env, &with_frp_context, 2);
        assert_eq!(vec![String::from("3 5"), String::from("6 10")], env.out);
    }

    #[test]
    fn lift_from_simultaneous() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<u32>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let (b1, b2) = FrpContext::transaction(
            &mut env,
            &with_frp_context,
            |env, with_frp_context| {
                let b1: CellSink<Env,u32>;
                let b2: CellSink<Env,u32>;
                {
                    let frp_context = with_frp_context.with_frp_context(env);
                    b1 = frp_context.new_cell_sink(3);
                    b2 = frp_context.new_cell_sink(5);
                }
                b2.change_value(env, with_frp_context, 7);
                (b1, b2)
            }
        );
        let c = env.frp_context.lift2_c(|x, y| x.clone() + y.clone(), &b1, &b2);
        c.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        assert_eq!(vec![10], env.out);
    }

    #[test]
    fn hold_is_delayed() {
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let s = env.frp_context.new_stream_sink();
        let h = env.frp_context.hold(0, &s);
        let sPair = env.frp_context.snapshot(|a, b| format!("{} {}", a, b), &s, &h);
        sPair.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        s.send(&mut env, &with_frp_context, 2);
        s.send(&mut env, &with_frp_context, 3);
        assert_eq!(vec![String::from("2 0"), String::from("3 2")], env.out);
    }

    #[test]
    fn switch_c() {
        struct SC {
            a: Option<String>,
            b: Option<String>,
            sw: Option<String>
        }
        impl SC {
            fn of(a: Option<String>, b: Option<String>, sw: Option<String>) -> SC {
                SC {
                    a: a,
                    b: b,
                    sw: sw
                }
            }
        }
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let ssc: StreamSink<Env,SC> = env.frp_context.new_stream_sink();
        let ca_1 = env.frp_context.map_s(&ssc, |s| s.a.clone());
        let ca_2 = env.frp_context.filter_some(&ca_1);
        let ca = env.frp_context.hold(String::from("A"), &ca_2);
        let cb_1 = env.frp_context.map_s(&ssc, |s| s.b.clone());
        let cb_2 = env.frp_context.filter_some(&cb_1);
        let cb = env.frp_context.hold(String::from("a"), &cb_2);
        let csw_str_1 = env.frp_context.map_s(&ssc, |s| s.sw.clone());
        let csw_str_2 = env.frp_context.filter_some(&csw_str_1);
        let csw_str = env.frp_context.hold(String::from("ca"), &csw_str_2);
        let csw: Cell<Env,Cell<Env,String>> = env.frp_context.map_c(&csw_str, move |s| if s == "ca" { ca } else { cb });
        let co_1 = env.frp_context.map_c(
            &csw,
            move |x| {
                let k: Box<Fn(&mut FrpContext<Env>)->Cell<Env,String>>;
                let x2 = x.clone();
                k = Box::new(move |_| { x2 });
                return k;
            }
        );
        let co = env.frp_context.switch_c(&co_1);
        co.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("B")), Some(String::from("b")), None));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("C")), Some(String::from("c")), Some(String::from("cb"))));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("D")), Some(String::from("d")), None));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("E")), Some(String::from("e")), Some(String::from("ca"))));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("F")), Some(String::from("f")), None));
        ssc.send(&mut env, &with_frp_context, SC::of(None, None, Some(String::from("cb"))));
        ssc.send(&mut env, &with_frp_context, SC::of(None, None, Some(String::from("ca"))));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("G")), Some(String::from("g")), Some(String::from("cb"))));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("H")), Some(String::from("h")), Some(String::from("ca"))));
        ssc.send(&mut env, &with_frp_context, SC::of(Some(String::from("I")), Some(String::from("i")), Some(String::from("ca"))));
        assert_eq!(vec![String::from("A"), String::from("B"), String::from("c"), String::from("d"), String::from("E"), String::from("F"), String::from("f"), String::from("F"), String::from("g"), String::from("H"), String::from("I")], env.out);
    }

    #[test]
    fn switch_s() {
        struct SS {
            a: Option<String>,
            b: Option<String>,
            sw: Option<String>
        }
        impl SS {
            fn of(a: Option<String>, b: Option<String>, sw: Option<String>) -> SS {
                SS {
                    a: a,
                    b: b,
                    sw: sw
                }
            }
        }
        struct Env {
            frp_context: FrpContext<Env>,
            out: Vec<String>
        }
        let mut env = Env { frp_context: FrpContext::new(), out: Vec::new() };
        #[derive(Copy,Clone)]
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<'r>(&self, env: &'r mut Env) -> &'r mut FrpContext<Env> {
                return &mut env.frp_context;
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let sss: StreamSink<Env,SS> = env.frp_context.new_stream_sink();
        let sa_1 = env.frp_context.map_s(&sss, |s| s.a.clone());
        let sa = env.frp_context.filter_some(&sa_1);
        let sb_1 = env.frp_context.map_s(&sss, |s| s.b.clone());
        let sb = env.frp_context.filter_some(&sb_1);
        let csw_str_1 = env.frp_context.map_s(&sss, |s| s.sw.clone());
        let csw_str_2 = env.frp_context.filter_some(&csw_str_1);
        let csw_str = env.frp_context.hold(String::from("sa"), &csw_str_2);
        let sa = sa.clone();
        let sb = sb.clone();
        let csw: Cell<Env,Stream<Env,String>> = env.frp_context.map_c(&csw_str, move |s| if s == "sa" { sa } else { sb });
        let so_1 = env.frp_context.map_c(
            &csw,
            move |x| {
                let k: Box<Fn(&mut FrpContext<Env>)->Stream<Env,String>>;
                let x2 = x.clone();
                k = Box::new(move |_| { x2 });
                return k;
            }
        );
        let so = env.frp_context.switch_s(&so_1);
        so.observe(&mut env, &with_frp_context, |env, value| env.out.push(value.clone()));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("A")), Some(String::from("a")), None));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("B")), Some(String::from("b")), None));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("C")), Some(String::from("c")), Some(String::from("sb"))));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("D")), Some(String::from("d")), None));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("E")), Some(String::from("e")), Some(String::from("sa"))));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("F")), Some(String::from("f")), None));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("G")), Some(String::from("g")), Some(String::from("sb"))));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("H")), Some(String::from("h")), Some(String::from("sa"))));
        sss.send(&mut env, &with_frp_context, SS::of(Some(String::from("I")), Some(String::from("i")), Some(String::from("sa"))));
        assert_eq!(vec![String::from("A"), String::from("B"), String::from("C"), String::from("d"), String::from("e"), String::from("F"), String::from("G"), String::from("h"), String::from("I")], env.out);
    }
/*
class SS2 {
    s : StreamSink<number> = new StreamSink<number>();
}

test("switchSSimultaneous", () => {
    const ss1 = new SS2(),
          ss2 = new SS2(),
          ss3 = new SS2(),
          ss4 = new SS2(),
          css = new CellSink<SS2>(ss1),
          // ****
          // NOTE! Because this lambda contains references to Sodium objects, we
          // must declare them explicitly using lambda1() so that Sodium knows
          // about the dependency, otherwise it can't manage the memory.
          // ****
          so = Cell.switchS(css.map(lambda1((b : SS2) => b.s, [ss1.s, ss2.s, ss3.s, ss4.s]))),
          out : number[] = [],
          kill = so.listen(c => out.push(c));
    ss1.s.send(0);
    ss1.s.send(1);
    ss1.s.send(2);
    css.send(ss2);
    ss1.s.send(7);
    ss2.s.send(3);
    ss2.s.send(4);
    ss3.s.send(2);
    css.send(ss3);
    ss3.s.send(5);
    ss3.s.send(6);
    ss3.s.send(7);
    Transaction.run(() => {
        ss3.s.send(8);
        css.send(ss4);
        ss4.s.send(2);
    });
    ss4.s.send(9);
    kill();
    assertEquals([0, 1, 2, 3, 4, 5, 6, 7, 8, 9], out);
});

test("loopCell", () => {
    const sa = new StreamSink<number>(),
        sum_out = Transaction.run(() => {
                const sum = new CellLoop<number>(),
                      sum_out_ = sa.snapshot(sum, (x, y) => x + y).hold(0);
                sum.loop(sum_out_);
                return sum_out_;
            }),
        out : number[] = [],
        kill = sum_out.listen(a => out.push(a));
    sa.send(2);
    sa.send(3);
    sa.send(1);
    kill();
    assertEquals([0, 2, 5, 6], out);
    assertEquals(6, sum_out.sample());
});

test("accum", () => {
    const sa = new StreamSink<number>(),
        out : number[] = [],
        sum = sa.accum(100, (a, s) => a + s),
        kill = sum.listen(a => out.push(a));
    sa.send(5);
    sa.send(7);
    sa.send(1);
    sa.send(2);
    sa.send(3);
    kill();
    assertEquals([100, 105, 112, 113, 115, 118], out);
});

test("loopValueSnapshot", () => {
    const out : string[] = [],
        kill = Transaction.run(() => {
            const a = new Cell("lettuce"),
               b = new CellLoop<string>(),
               eSnap = Operational.value(a).snapshot(b, (aa, bb) => aa + " " + bb);
            b.loop(new Cell("cheese"));
            return eSnap.listen(x => out.push(x));
        });
    kill();
    assertEquals(["lettuce cheese"], out);
});

test("loopValueHold", () => {
    const out : string[] = [],
        value = Transaction.run(() => {
            const a = new CellLoop<string>(),
                value_ = Operational.value(a).hold("onion");
            a.loop(new Cell("cheese"));
            return value_;
        }),
        sTick = new StreamSink<Unit>(),
        kill = sTick.snapshot1(value).listen(x => out.push(x));
    sTick.send(Unit.UNIT);
    kill();
    assertEquals(["cheese"], out);
});

test("liftLoop", () => {
    const out : string[] = [],
        b = new CellSink("kettle"),
        c = Transaction.run(() => {
            const a = new CellLoop<string>(),
                c_ = a.lift(b, (aa, bb) => aa + " " + bb);
            a.loop(new Cell("tea"));
            return c_;
        }),
        kill = c.listen(x => out.push(x));
    b.send("caddy");
    kill();
    assertEquals(["tea kettle", "tea caddy"], out);
});

const name = "fromAsync",
     action = IOAction.fromAsync((a : number, result : (b : number) => void) => {
            setTimeout(() => {
                    result(a + 1);
                }, 1);
        }),
     out : number[] = [],
     sa = new StreamSink<number>(),
     kill = action(sa).listen(b => out.push(b));
sa.send(5);
assertEquals([], out);
setTimeout(() => {
        sa.send(9);
        assertEquals([6], out);
        setTimeout(() => {
            assertEquals([6, 10], out);
            kill();
            checkMemory();
            console.log(name + " - PASS");
        }, 100);
    }, 100);
*/

*/
}
