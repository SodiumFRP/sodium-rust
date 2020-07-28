use crate::{lambda1, Cell, CellLoop, Operational, SodiumCtx, Stream, StreamSink};

use std::sync::{Arc, Mutex};

mod mem_test;
mod node_test;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn assert_memory_freed(sodium_ctx: &SodiumCtx) {
    sodium_ctx.impl_.collect_cycles();
    let node_count = sodium_ctx.impl_.node_count();
    let node_ref_count = sodium_ctx.impl_.node_ref_count();
    println!();
    println!("node_count {}", node_count);
    println!("node_ref_count {}", node_ref_count);
    assert_eq!(node_count, 0);
}

#[test]
fn stream() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let ev = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(String::new()));
        let l;
        {
            let out = out.clone();
            l = sodium_ctx.transaction(|| {
                ev.send("h");
                let l = ev
                    .stream()
                    .listen(move |ch: &&'static str| out.lock().as_mut().unwrap().push_str(ch));
                ev.send("e");
                l
            });
        }
        sodium_ctx.transaction(|| {
            ev.send("l");
            ev.send("l");
            ev.send("o");
        });
        l.unlisten();
        ev.send("!");
        {
            let l = out.lock();
            let out: &String = l.as_ref().unwrap();
            assert_eq!(String::from("eo"), *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn map() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s: StreamSink<i32> = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .map(|a: &i32| *a + 1)
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(a.clone()));
        }
        s.send(7);
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![8], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

// TODO: missing map_optional

#[test]
fn merge_non_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink();
        let s2 = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s2
                .stream()
                .or_else(&s1.stream())
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        s1.send(7);
        s2.send(9);
        s1.send(8);
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![7, 9, 8], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn filter() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .filter(|a: &u32| *a < 10)
                .listen(move |a: &u32| out.lock().as_mut().unwrap().push(*a));
        }
        s.send(2);
        s.send(16);
        s.send(9);
        {
            let lock = out.lock();
            let out: &Vec<u32> = lock.as_ref().unwrap();
            assert_eq!(vec![2, 9], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn filter_option() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s: StreamSink<Option<&'static str>> = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .filter_option()
                .listen(move |a: &&'static str| out.lock().as_mut().unwrap().push(*a));
        }
        s.send(Some("tomato"));
        s.send(None);
        s.send(Some("peach"));
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["tomato", "peach"], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_stream1() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let _sb = sodium_ctx.transaction(|| {
            let sb = sodium_ctx.new_stream_loop();
            sb.loop_(&sa.stream());
            sb
        });
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sa
                .stream()
                .listen(move |a: &u8| out.lock().as_mut().unwrap().push(*a));
        }
        sa.send(2);
        sa.send(52);
        {
            let l = out.lock();
            let out: &Vec<_> = l.as_ref().unwrap();
            assert_eq!(vec![2, 52], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_stream2() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sc = sodium_ctx.transaction(|| {
            let sb = sodium_ctx.new_stream_loop();
            let sc_ = sa
                .stream()
                .map(|x: &i32| *x % 10)
                .merge(&sb.stream(), |x: &i32, y: &i32| *x + *y);
            let sb_out = sa.stream().map(|x: &i32| *x / 10).filter(|x: &i32| *x != 0);
            sb.loop_(&sb_out);
            sc_
        });
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        sa.send(2);
        sa.send(52);
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![2, 7], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn gate() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let pred = sodium_ctx.new_cell_sink(true);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .gate(&pred.cell())
                .listen(move |a: &&'static str| out.lock().as_mut().unwrap().push(*a));
        }
        s.send("H");
        pred.send(false);
        s.send("O");
        pred.send(true);
        s.send("I");
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["H", "I"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn once() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .once()
                .listen(move |a: &&'static str| out.lock().as_mut().unwrap().push(*a));
        }
        s.send("A");
        s.send("B");
        s.send("C");
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["A"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn hold() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.stream().hold(0);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Operational::updates(&c)
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        s.send(2);
        s.send(9);
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![2, 9], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn snapshot() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink::<usize>();
        let c = sodium_ctx.new_cell_sink(0);

        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .snapshot(&c.cell(), |x: &usize, y: &usize| format!("{} {}", x, y))
                .listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        s.send(100);
        c.send(2);
        s.send(200);
        c.send(9);
        c.send(1);
        s.send(300);
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(
                vec!["100 0", "200 2", "300 1"],
                out.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
            );
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn snapshot3() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink::<usize>();
        let b = sodium_ctx.new_cell_sink(0);
        let c = sodium_ctx.new_cell_sink(5);

        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .snapshot3(&b.cell(), &c.cell(), |x: &usize, y: &usize, z: &usize| {
                    format!("{} {} {}", x, y, z)
                })
                .listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        s.send(100);
        b.send(2);
        s.send(200);
        b.send(9);
        b.send(1);
        s.send(300);
        c.send(3);
        s.send(400);
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(
                vec!["100 0 5", "200 2 5", "300 1 5", "400 1 3"],
                out.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
            );
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

fn deltas_with_initial<A, F, R>(sodium_ctx: &SodiumCtx, ca: &Cell<A>, a: A, f: F) -> Stream<R>
where
    A: 'static + Clone + Send,
    R: 'static + Clone + Send,
    F: 'static + Send + Sync + Fn(&A, &A) -> R,
{
    sodium_ctx.transaction(|| {
        let s = ca.value();
        let previous = s.hold(a);
        s.snapshot(&previous, f)
    })
}

#[test]
fn snapshot_initial_value() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let (sa, l) = sodium_ctx.transaction(|| {
            let sa = sodium_ctx.new_stream_sink();
            let a = sa.stream().hold(5);
            sa.send(10);
            let l;
            {
                let out = out.clone();
                l = deltas_with_initial(sodium_ctx, &a, 0, |new: &i8, old: &i8| {
                    println!("new {} old {}", new, old);
                    new - old
                })
                .listen(move |a: &i8| out.lock().as_mut().unwrap().push(a.clone()));
            }
            (sa, l)
        });
        sa.send(12);
        sa.send(30);
        {
            let l = out.lock();
            let out: &Vec<_> = l.as_ref().unwrap();
            assert_eq!(vec![10, 2, 18], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn value() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let (b, l) = sodium_ctx.transaction(|| {
            let b = sodium_ctx.new_cell_sink(9);
            let l;
            {
                let out = out.clone();
                l = b
                    .cell()
                    .value()
                    .listen(move |a: &i8| out.lock().as_mut().unwrap().push(a.clone()));
            }
            (b, l)
        });
        b.send(2);
        b.send(7);
        {
            let l = out.lock();
            let out: &Vec<_> = l.as_ref().unwrap();
            assert_eq!(vec![9, 2, 7], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn value_const() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let l = sodium_ctx.transaction(|| {
            let b = sodium_ctx.new_cell_sink(9);
            let l;
            {
                let out = out.clone();
                l = b
                    .cell()
                    .value()
                    .listen(move |a: &i8| out.lock().as_mut().unwrap().push(a.clone()));
            }
            l
        });
        {
            let l = out.lock();
            let out: &Vec<_> = l.as_ref().unwrap();
            assert_eq!(vec![9], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

// TARGET

#[test]
fn constant_cell() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let c = sodium_ctx.new_cell(12);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = c.listen(move |a: &i32| out.lock().as_mut().unwrap().push(a.clone()));
        }
        {
            let l = out.lock();
            let out: &Vec<i32> = l.as_ref().unwrap();
            assert_eq!(vec![12], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn values() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let c = sodium_ctx.new_cell_sink(9_i32);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = c
                .cell()
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(a.clone()));
        }
        c.send(2);
        c.send(7);
        {
            let l = out.lock();
            let out: &Vec<i32> = l.as_ref().unwrap();
            assert_eq!(vec![9, 2, 7], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn map_c() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let c = sodium_ctx.new_cell_sink(6);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = c
                .cell()
                .map(|a: &i32| format!("{}", a))
                .listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        c.send(8);
        l.unlisten();
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(vec![String::from("6"), String::from("8")], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift_cells_in_switch_c() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let s = sodium_ctx.new_cell_sink(0);
        let c = sodium_ctx.new_cell(sodium_ctx.new_cell(1));
        let r;
        {
            let s = s.clone();
            r = c.map(move |c2: &Cell<i32>| c2.lift2(&s.cell(), |v1: &i32, v2: &i32| *v1 + *v2));
        }
        {
            let out = out.clone();
            l = Cell::switch_c(&r).listen(move |a: &i32| {
                out.lock().as_mut().unwrap().push(*a);
            });
        }
        s.send(2);
        s.send(4);
        {
            let l = out.lock();
            let out: &Vec<i32> = l.as_ref().unwrap();
            assert_eq!(vec![1, 3, 5], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn send_before_listen() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let c = sodium_ctx.new_cell_sink(9_i32);
        let cm = c.cell().map(|a: &i32| format!("{}", a));
        c.send(2);
        {
            let out = out.clone();
            l = cm.listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
            c.send(8);
        }
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(
                vec!["2", "8"],
                out.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
            );
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let a = sodium_ctx.new_cell_sink(1);
        let b = sodium_ctx.new_cell_sink(5);
        {
            let out = out.clone();
            l = a
                .cell()
                .lift2(&b.cell(), |aa: &i32, bb: &i32| format!("{} {}", aa, bb))
                .listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        a.send(12);
        b.send(6);
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(
                vec!["1 5", "12 5", "12 6"],
                out.iter().map(|s| s.as_str()).collect::<Vec<&str>>()
            );
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift_glitch() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let a = sodium_ctx.new_cell_sink(1);
        let ac = a.cell();
        let a3 = ac.map(|x: &i32| x * 3);
        let a5 = ac.map(|x: &i32| x * 5);
        let b = a3.lift2(&a5, |x: &i32, y: &i32| format!("{} {}", x, y));
        {
            let out = out.clone();
            l = b.listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        a.send(2);
        {
            let l = out.lock();
            let out: &Vec<String> = l.as_ref().unwrap();
            assert_eq!(vec!["3 5", "6 10"], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn lift_from_simultaneous() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let (b1, b2) = sodium_ctx.transaction(|| {
            let b1 = sodium_ctx.new_cell_sink(3);
            let b2 = sodium_ctx.new_cell_sink(5);
            b2.send(7);
            (b1, b2)
        });
        {
            let out = out.clone();
            l = b1
                .cell()
                .lift2(&b2.cell(), |x: &i32, y: &i32| x + y)
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(a.clone()));
        }
        {
            let l = out.lock();
            let out: &Vec<i32> = l.as_ref().unwrap();
            assert_eq!(vec![10], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn router1() {
    #[derive(Clone)]
    pub struct Packet {
        pub address: i32,
        pub payload: &'static str,
    }
    impl Packet {
        fn new(address: i32, payload: &'static str) -> Packet {
            Packet { address, payload }
        }
    }
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink::<Packet>();
        let r = sodium_ctx.new_router(&s.stream(), |pkt: &Packet| vec![pkt.address]);
        let one = r.filter_matches(&1);
        let out_one = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let kill_one;
        {
            let out_one = out_one.clone();
            kill_one =
                one.listen(move |p: &Packet| out_one.lock().as_mut().unwrap().push(p.payload));
        }
        let two = r.filter_matches(&2);
        let out_two = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let kill_two;
        {
            let out_two = out_two.clone();
            kill_two =
                two.listen(move |p: &Packet| out_two.lock().as_mut().unwrap().push(p.payload));
        }
        let three = r.filter_matches(&3);
        let out_three = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let kill_three;
        {
            let out_three = out_three.clone();
            kill_three =
                three.listen(move |p: &Packet| out_three.lock().as_mut().unwrap().push(p.payload));
        }
        s.send(Packet::new(1, "dog"));
        s.send(Packet::new(3, "manuka"));
        s.send(Packet::new(2, "square"));
        s.send(Packet::new(3, "tawa"));
        s.send(Packet::new(2, "circle"));
        s.send(Packet::new(1, "otter"));
        s.send(Packet::new(1, "lion"));
        s.send(Packet::new(2, "rectangle"));
        s.send(Packet::new(3, "rata"));
        s.send(Packet::new(4, "kauri"));
        kill_one.unlisten();
        kill_two.unlisten();
        kill_three.unlisten();
        {
            let l1 = out_one.lock();
            let out_one: &Vec<&'static str> = l1.as_ref().unwrap();
            let l2 = out_two.lock();
            let out_two: &Vec<&'static str> = l2.as_ref().unwrap();
            let l3 = out_three.lock();
            let out_three: &Vec<&'static str> = l3.as_ref().unwrap();
            assert_eq!(vec!["dog", "otter", "lion"], *out_one);
            assert_eq!(vec!["square", "circle", "rectangle"], *out_two);
            assert_eq!(vec!["manuka", "tawa", "rata"], *out_three);
        }
    }
}

// TODO(RadicalZephyr 2020-07-17): port apply and then uncomment this test
// #[test]
// fn apply() {
//     let sodium_ctx = SodiumCtx::new();
//     let sodium_ctx = &sodium_ctx;
//     let l;
//     {
//         let out = Arc::new(Mutex::new(Vec::new()));
//         let cf = sodium_ctx.new_cell_sink(Box::new(|a: &u8| format!("1 {}", a)) as Box<dyn Fn(&u8) -> String>);
//         let ca = sodium_ctx.new_cell_sink(5);

//         {
//             let out = out.clone();
//             l = Cell::apply(cf, ca).listen(move |a: &i32| out.lock().as_mut().unwrap().push(a.clone()));
//         }
//         cf.send(Box::new(|a: &u8| format!("12 {}", a)) as Box<dyn Fn(&u8) -> String>);
//         ca.send(6);
//         {
//             let l = out.lock();
//             let out: &Vec<i32> = l.as_ref().unwrap();
//             assert_eq!(vec!["1 5", "12 5", "12 6"], *out);
//         }
//     }
//     l.unlisten();
//     assert_memory_freed(sodium_ctx);
// }

#[test]
fn loop_value_snapshot() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sodium_ctx.transaction(|| {
                let a = sodium_ctx.new_cell("lettuce");
                let b = sodium_ctx.new_cell_loop();
                let e_snap = Operational::value(&a)
                    .snapshot(&b.cell(), |aa: &&str, bb: &&str| format!("{} {}", aa, bb));
                b.loop_(&sodium_ctx.new_cell("cheese"));
                e_snap.listen(move |x: &String| out.lock().as_mut().unwrap().push(x.clone()))
            });
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
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let value = sodium_ctx.transaction(|| {
            let a = sodium_ctx.new_cell_loop();
            let value_ = Operational::value(&a.cell()).hold("onion");
            a.loop_(&sodium_ctx.new_cell("cheese"));
            value_
        });
        let s_tick = sodium_ctx.new_stream_sink();
        let l;
        {
            let out = out.clone();
            l = s_tick
                .stream()
                .snapshot1(&value)
                .listen(move |x: &&'static str| out.lock().as_mut().unwrap().push(x.clone()));
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
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::new()));
        let b = sodium_ctx.new_cell_sink("kettle");
        let c = sodium_ctx.transaction(|| {
            let a = sodium_ctx.new_cell_loop();
            let c_ = a
                .cell()
                .lift2(&b.cell(), |aa: &&'static str, bb: &&'static str| {
                    format!("{} {}", aa, bb)
                });
            a.loop_(&sodium_ctx.new_cell("tea"));
            c_
        });
        let l;
        {
            let out = out.clone();
            l = c.listen(move |x: &String| out.lock().as_mut().unwrap().push(x.clone()));
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

#[test]
fn loop_switch_s() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::<&str>::new()));
        let (b, listener) = sodium_ctx.transaction(|| {
            let e1: StreamSink<&str> = sodium_ctx.new_stream_sink();
            let b_lp: CellLoop<Stream<&str>> = sodium_ctx.new_cell_loop();
            let e: Stream<&str> = Cell::switch_s(&b_lp.cell());
            e1.send("banana");
            let out = out.clone();
            let listener = e.listen(move |x: &_| out.lock().as_mut().unwrap().push(*x));
            let b = sodium_ctx.new_cell_sink(e1.stream());
            b_lp.loop_(&b.cell());
            (b, listener)
        });
        let e2 = sodium_ctx.new_stream_sink();
        e2.send("peer");
        b.send(e2.stream());
        e2.send("apple");
        listener.unlisten();
        {
            let l = out.lock();
            let out: &Vec<&str> = l.as_ref().unwrap();
            assert_eq!(vec!["banana", "apple"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn map_to() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        {
            let out = out.clone();
            l = s
                .stream()
                .map_to("fusebox")
                .listen(move |a: &&'static str| out.lock().as_mut().unwrap().push(*a));
        }
        s.send(7);
        s.send(9);
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["fusebox", "fusebox"], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn merge_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let s2 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s2
                .stream()
                .or_else(&s1.stream())
                .listen(move |a: &i32| (*out).lock().as_mut().unwrap().push(*a));
        }
        sodium_ctx.transaction(|| {
            s1.send(7);
            s2.send(60);
        });
        sodium_ctx.transaction(|| {
            s1.send(9);
        });
        sodium_ctx.transaction(|| {
            s1.send(7);
            s1.send(60);
            s2.send(8);
            s2.send(90);
        });
        sodium_ctx.transaction(|| {
            s2.send(8);
            s2.send(90);
            s1.send(7);
            s1.send(60);
        });
        sodium_ctx.transaction(|| {
            s2.send(8);
            s1.send(7);
            s2.send(90);
            s1.send(60);
        });
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![60, 9, 90, 90, 90], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn coalesce() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink_with_coalescer(|a, b| *a + *b);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        sodium_ctx.transaction(|| {
            s.send(2);
        });
        sodium_ctx.transaction(|| {
            s.send(8);
            s.send(40);
        });
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![2, 48], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn merge() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sb = sa.stream().map(|x: &i32| *x / 10).filter(|x: &i32| *x != 0);
        let sc = sa
            .stream()
            .map(|x: &i32| *x % 10)
            .merge(&sb, |x: &i32, y: &i32| *x + *y);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        sa.send(2);
        sa.send(52);
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![2, 7], *out);
        }
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn collect() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let sum = ea
            .stream()
            .collect(0, |a: &u32, s: &u32| (*a + *s + 100, *a + *s));
        {
            let out = out.clone();
            l = sum.listen(move |a: &u32| out.lock().as_mut().unwrap().push(*a));
        }
        ea.send(5);
        ea.send(7);
        ea.send(1);
        ea.send(2);
        ea.send(3);
        {
            let lock = out.lock();
            let out: &Vec<u32> = lock.as_ref().unwrap();
            assert_eq!(vec![105, 112, 113, 115, 118], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn accum() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let sum = ea.stream().accum(100, |a: &u32, s: &u32| *a + *s);
        {
            let out = out.clone();
            l = sum.listen(move |a: &u32| out.lock().as_mut().unwrap().push(*a));
        }
        ea.send(5);
        ea.send(7);
        ea.send(1);
        ea.send(2);
        ea.send(3);
        {
            let lock = out.lock();
            let out: &Vec<u32> = lock.as_ref().unwrap();
            assert_eq!(vec![100, 105, 112, 113, 115, 118], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn split1() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let out = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let ea = sodium_ctx.new_stream_sink::<&'static str>();
        let eo = ea
            .stream()
            .map(|text0: &&'static str| text0.split(" "))
            .split();
        let listener;
        {
            let out = out.clone();
            listener = eo.listen(move |x: &&'static str| {
                out.lock().as_mut().unwrap().push(*x);
            });
        }
        ea.send("the common cormorant");
        ea.send("or shag");
        listener.unlisten();
        {
            let l = out.lock();
            let out: &Vec<&'static str> = l.as_ref().unwrap();
            assert_eq!(vec!["the", "common", "cormorant", "or", "shag"], *out)
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn defer() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.stream().hold(" ");
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Operational::defer(&s.stream())
                .snapshot1(&c)
                .listen(move |a: &&'static str| out.lock().as_mut().unwrap().push(*a));
        }
        s.send("C");
        s.send("B");
        s.send("A");
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["C", "B", "A"], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn hold_is_delayed() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let h = s.stream().hold(0);
        let s_pair = s
            .stream()
            .snapshot(&h, |a: &i32, b: &i32| format!("{} {}", *a, *b));
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s_pair.listen(move |a: &String| out.lock().as_mut().unwrap().push(a.clone()));
        }
        s.send(2);
        s.send(3);
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<String> = lock.as_ref().unwrap();
            assert_eq!(vec![String::from("2 0"), String::from("3 2")], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_c() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        #[derive(Clone)]
        struct SC {
            a: Option<&'static str>,
            b: Option<&'static str>,
            sw: Option<&'static str>,
        }
        impl SC {
            fn new(
                a: Option<&'static str>,
                b: Option<&'static str>,
                sw: Option<&'static str>,
            ) -> SC {
                SC { a: a, b: b, sw: sw }
            }
        }
        let ssc = sodium_ctx.new_stream_sink();
        let ca = ssc
            .stream()
            .map(|s: &SC| s.a.clone())
            .filter_option()
            .hold("A");
        let cb = ssc
            .stream()
            .map(|s: &SC| s.b.clone())
            .filter_option()
            .hold("a");
        let csw_str = ssc
            .stream()
            .map(|s: &SC| s.sw.clone())
            .filter_option()
            .hold("ca");
        let csw_deps = vec![ca.to_dep(), cb.to_dep()];
        let csw = csw_str.map(lambda1(
            move |s: &&'static str| if *s == "ca" { ca.clone() } else { cb.clone() },
            csw_deps,
        ));
        let co = Cell::switch_c(&csw);
        let out = Arc::new(Mutex::new(Vec::new()));
        {
            let out = out.clone();
            l = co.listen(move |c: &&'static str| out.lock().as_mut().unwrap().push(*c));
        }
        ssc.send(SC::new(Some("B"), Some("b"), None));
        ssc.send(SC::new(Some("C"), Some("c"), Some("cb")));
        ssc.send(SC::new(Some("D"), Some("d"), None));
        ssc.send(SC::new(Some("E"), Some("e"), Some("ca")));
        ssc.send(SC::new(Some("F"), Some("f"), None));
        ssc.send(SC::new(None, None, Some("cb")));
        ssc.send(SC::new(None, None, Some("ca")));
        ssc.send(SC::new(Some("G"), Some("g"), Some("cb")));
        ssc.send(SC::new(Some("H"), Some("h"), Some("ca")));
        ssc.send(SC::new(Some("I"), Some("i"), Some("ca")));
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(
                vec!["A", "B", "c", "d", "E", "F", "f", "F", "g", "H", "I"],
                *out
            );
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_s() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    let l;
    {
        #[derive(Clone)]
        struct SS {
            a: &'static str,
            b: &'static str,
            sw: Option<&'static str>,
        }
        impl SS {
            fn new(a: &'static str, b: &'static str, sw: Option<&'static str>) -> SS {
                SS { a: a, b: b, sw: sw }
            }
        }
        let sss = sodium_ctx.new_stream_sink();
        let sa = sss.stream().map(|s: &SS| s.a.clone());
        let sb = sss.stream().map(|s: &SS| s.b.clone());
        let csw_str = sss
            .stream()
            .map(|s: &SS| s.sw.clone())
            .filter_option()
            .hold("sa");
        let csw_deps = vec![sa.to_dep(), sb.to_dep()];
        let csw: Cell<Stream<&'static str>> = csw_str.map(lambda1(
            move |sw: &&'static str| if *sw == "sa" { sa.clone() } else { sb.clone() },
            csw_deps,
        ));
        let so = Cell::switch_s(&csw);
        let out = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        {
            let out = out.clone();
            l = so.listen(move |x: &&'static str| out.lock().as_mut().unwrap().push(*x));
        }
        sss.send(SS::new("A", "a", None));
        sss.send(SS::new("B", "b", None));
        sss.send(SS::new("C", "c", Some("sb")));
        sss.send(SS::new("D", "d", None));
        sss.send(SS::new("E", "e", Some("sa")));
        sss.send(SS::new("F", "f", None));
        sss.send(SS::new("G", "g", Some("sb")));
        sss.send(SS::new("H", "h", Some("sa")));
        sss.send(SS::new("I", "i", Some("sa")));
        {
            let lock = out.lock();
            let out: &Vec<&'static str> = lock.as_ref().unwrap();
            assert_eq!(vec!["A", "B", "C", "d", "e", "F", "G", "h", "I"], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_s_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        #[derive(Clone)]
        struct SS2 {
            s: StreamSink<i32>,
        }
        impl SS2 {
            fn new(sodium_ctx: &SodiumCtx) -> SS2 {
                SS2 {
                    s: sodium_ctx.new_stream_sink(),
                }
            }
        }
        let ss1 = SS2::new(sodium_ctx);
        let ss2 = SS2::new(sodium_ctx);
        let ss3 = SS2::new(sodium_ctx);
        let ss4 = SS2::new(sodium_ctx);
        let css = sodium_ctx.new_cell_sink(ss1.clone());
        let so = Cell::switch_s(&css.cell().map(|b: &SS2| b.s.stream()));
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = so.listen(move |c: &i32| out.lock().as_mut().unwrap().push(*c));
        }
        ss1.s.send(0);
        ss1.s.send(1);
        ss1.s.send(2);
        css.send(ss2.clone());
        ss1.s.send(7);
        ss2.s.send(3);
        ss2.s.send(4);
        ss3.s.send(2);
        css.send(ss3.clone());
        ss3.s.send(5);
        ss3.s.send(6);
        ss3.s.send(7);
        sodium_ctx.transaction(|| {
            ss3.s.send(8);
            css.send(ss4.clone());
            ss4.s.send(2);
        });
        ss4.s.send(9);
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], *out);
        }
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_cell() {
    init();
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sum_out = sodium_ctx.transaction(|| {
            let sum = sodium_ctx.new_cell_loop();
            let sum_out = sa
                .stream()
                .snapshot(&sum.cell(), |x: &i32, y: &i32| *x + *y)
                .hold(0);
            sum.loop_(&sum_out);
            sum_out
        });
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sum_out.listen(move |a: &i32| out.lock().as_mut().unwrap().push(*a));
        }
        sa.send(2);
        sa.send(3);
        sa.send(1);
        l.unlisten();
        {
            let lock = out.lock();
            let out: &Vec<i32> = lock.as_ref().unwrap();
            assert_eq!(vec![0, 2, 5, 6], *out);
        }
        assert_eq!(6, sum_out.sample());
    }
    assert_memory_freed(sodium_ctx);
}
