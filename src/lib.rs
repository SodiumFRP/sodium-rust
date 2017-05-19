extern crate topological_sort;

pub mod rusty_frp;

#[cfg(test)]
mod tests {
    use rusty_frp::Cell;
    use rusty_frp::CellSink;
    use rusty_frp::CellTrait;
    use rusty_frp::FrpContext;
    use rusty_frp::StreamTrait;
    use rusty_frp::WithFrpContext;

    #[test]
    fn test_via_console() {
        // use: cargo test -- --nocapture
        test_cell_sink();
        test_cell_map();
        test_stream_map();
        test_lift2();
        test_cell_loop();
    }

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
}
