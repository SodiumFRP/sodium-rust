extern crate topological_sort;

pub mod rusty_frp;

#[cfg(test)]
mod tests {
    use rusty_frp::FrpContext;
    use rusty_frp::WithFrpContext;

    #[test]
    fn test_via_console() {
        // use: cargo test -- --nocapture
        test_map();
        test_lift2();
    }

    fn test_map() {
        println!("test_map");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<F,R>(&self, env: &mut Env, k: F) -> R
            where F: FnOnce(&mut FrpContext<Env>) -> R {
                k(&mut env.frp_context)
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs1 = FrpContext::new_cell_sink(&mut env, &with_frp_context, 1u32);
        let c2 = FrpContext::map_cell(&mut env, &with_frp_context, &cs1, |value| { value + 1 });
        c2.observe(&mut env, &with_frp_context, |_, value| { println!("c2 = {}", value); });
        cs1.change_value(&mut env, &with_frp_context, 2);
        cs1.change_value(&mut env, &with_frp_context, 3);
        cs1.change_value(&mut env, &with_frp_context, 4);
    }

    fn test_lift2() {
        println!("test_lift2");
        struct Env {
            frp_context: FrpContext<Env>
        }
        let mut env = Env { frp_context: FrpContext::new() };
        struct WithFrpContextForEnv {}
        impl WithFrpContext<Env> for WithFrpContextForEnv {
            fn with_frp_context<F,R>(&self, env: &mut Env, k: F) -> R
            where F: FnOnce(&mut FrpContext<Env>) -> R {
                k(&mut env.frp_context)
            }
        }
        let with_frp_context = WithFrpContextForEnv {};
        let cs1 = FrpContext::new_cell_sink(&mut env, &with_frp_context, 1u32);
        let cs2 = FrpContext::new_cell_sink(&mut env, &with_frp_context, 1u32);
        let c3 =
            FrpContext::lift2_cell(
                &mut env,
                &with_frp_context,
                |a, b| a + b,
                &cs1,
                &cs2
            );
        c3.observe(&mut env, &with_frp_context, |_, value| { println!("c3 = {}", value); });
        cs1.change_value(&mut env, &with_frp_context, 2);
        cs2.change_value(&mut env, &with_frp_context, 3);
        cs1.change_value(&mut env, &with_frp_context, 4);
    }
}
