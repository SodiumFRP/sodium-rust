use crate::Cell;
use crate::lambda1;
use crate::Operational;
use crate::SodiumCtx;
use crate::Stream;
use crate::StreamSink;
use crate::tests::assert_memory_freed;
use crate::tests::init;

use std::sync::Arc;
use std::sync::Mutex;

#[test]
fn map() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s: StreamSink<i32> = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.stream().map(|a: &i32| *a + 1)
                .listen(
                    move |a: &i32| {
                        out.lock().as_mut().unwrap().push(a.clone())
                    }
                );
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

#[test]
fn map_to() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        {
            let out = out.clone();
            l =
                s.stream().map_to("fusebox")
                    .listen(
                        move |a: &&'static str| {
                            out.lock().as_mut().unwrap().push(*a)
                        }
                    );
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
fn merge_non_simultaneous() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink();
        let s2 = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.stream().or_else(&s1.stream())
                    .listen(
                        move |a: &i32|
                            out.lock().as_mut().unwrap().push(*a)
                    );
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
fn merge_simultaneous() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let s2 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.stream().or_else(&s1.stream())
                    .listen(
                        move |a: &i32|
                            (*out).lock().as_mut().unwrap().push(*a)
                    );
        }
        sodium_ctx.transaction(
            || {
                s1.send(7);
                s2.send(60);
            }
        );
        sodium_ctx.transaction(
            || {
                s1.send(9);
            }
        );
        sodium_ctx.transaction(
            || {
                s1.send(7);
                s1.send(60);
                s2.send(8);
                s2.send(90);
            }
        );
        sodium_ctx.transaction(
            || {
                s2.send(8);
                s2.send(90);
                s1.send(7);
                s1.send(60);
            }
        );
        sodium_ctx.transaction(
            || {
                s2.send(8);
                s1.send(7);
                s2.send(90);
                s1.send(60);
            }
        );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink_with_coalescer(|a, b| *a + *b);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.stream().listen(
                move |a: &i32|
                    out.lock().as_mut().unwrap().push(*a)
            );
        }
        sodium_ctx.transaction(
            || {
                s.send(2);
            }
        );
        sodium_ctx.transaction(
            || {
                s.send(8);
                s.send(40);
            }
        );
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
fn filter() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .filter(|a: &u32| *a < 10)
                .listen(
                    move |a: &u32|
                        out.lock().as_mut().unwrap().push(*a)
                );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s: StreamSink<Option<&'static str>> = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .stream()
                .filter_option()
                .listen(
                    move |a: &&'static str|
                        out.lock().as_mut().unwrap().push(*a)
                );
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
fn merge() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sb =
            sa
                .stream()
                .map(|x: &i32| *x / 10)
                .filter(|x: &i32| *x != 0);
        let sc =
            sa
                .stream()
                .map(|x: &i32| *x % 10)
                .merge(&sb, |x: &i32, y: &i32| *x + *y);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                move |a: &i32|
                    out.lock().as_mut().unwrap().push(*a)
            );
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
fn loop_() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sc = sodium_ctx.transaction(
            || {
                let sb = sodium_ctx.new_stream_loop();
                let sc_ =
                    sa
                        .stream()
                        .map(|x: &i32| *x % 10)
                        .merge(&sb.stream(), |x: &i32, y: &i32| *x + *y);
                let sb_out =
                    sa
                        .stream()
                        .map(|x: &i32| *x / 10)
                        .filter(|x: &i32| *x != 0);
                sb.loop_(&sb_out);
                sc_
            }
        );
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                move |a: &i32|
                    out.lock().as_mut().unwrap().push(*a)
            );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let pred = sodium_ctx.new_cell_sink(true);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .stream()
                    .gate(&pred.cell())
                    .listen(
                        move |a: &&'static str|
                            out.lock().as_mut().unwrap().push(*a)
                    );
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
fn collect() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let sum = ea.stream().collect(0, |a:&u32,s:&u32| (*a + *s + 100, *a + *s));
        {
            let out = out.clone();
            l =
                sum.listen(
                    move |a: &u32|
                        out.lock().as_mut().unwrap().push(*a)
                );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let sum = ea.stream().accum(100, |a:&u32, s:&u32| *a + *s);
        {
            let out = out.clone();
            l =
                sum.listen(
                    move |a: &u32|
                        out.lock().as_mut().unwrap().push(*a)
                );
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
fn once() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .stream()
                    .once()
                    .listen(
                        move |a: &&'static str|
                            out.lock().as_mut().unwrap().push(*a)
                    );
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
fn defer() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.stream().hold(" ");
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                Operational
                    ::defer(&s.stream())
                    .snapshot1(&c)
                    .listen(
                        move |a: &&'static str|
                            out.lock().as_mut().unwrap().push(*a)
                    );
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
fn hold() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.stream().hold(0);
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Operational
                ::updates(&c)
                .listen(
                    move |a: &i32|
                        out.lock().as_mut().unwrap().push(*a)
                );
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
fn hold_is_delayed() {
    init();
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let h = s.stream().hold(0);
        let s_pair = s.stream().snapshot(&h, |a: &i32, b: &i32| format!("{} {}", *a, *b));
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s_pair
                    .listen(
                        move |a: &String|
                            out.lock().as_mut().unwrap().push(a.clone())
                    );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        #[derive(Clone)]
        struct SC {
            a: Option<&'static str>,
            b: Option<&'static str>,
            sw: Option<&'static str>
        }
        impl SC {
            fn new(a: Option<&'static str>, b: Option<&'static str>, sw: Option<&'static str>) -> SC {
                SC {
                    a: a,
                    b: b,
                    sw: sw
                }
            }
        }
        let ssc = sodium_ctx.new_stream_sink();
        let ca = ssc.stream().map(|s: &SC| s.a.clone()).filter_option().hold("A");
        let cb = ssc.stream().map(|s: &SC| s.b.clone()).filter_option().hold("a");
        let csw_str = ssc.stream().map(|s: &SC| s.sw.clone()).filter_option().hold("ca");
        let csw_deps = vec![ca.to_dep(), cb.to_dep()];
        let csw = csw_str.map(
            lambda1(
                move |s: &&'static str|
                    if *s == "ca" { ca.clone() } else { cb.clone() },
                csw_deps
            )
        );
        let co = Cell::switch_c(&csw);
        let out = Arc::new(Mutex::new(Vec::new()));
        {
            let out = out.clone();
            l =
                co.listen(
                    move |c: &&'static str|
                        out.lock().as_mut().unwrap().push(*c)
                );
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
            assert_eq!(vec!["A", "B", "c", "d", "E", "F", "f", "F", "g", "H", "I"], *out);
        }
    }
    l.unlisten();
    assert_memory_freed(sodium_ctx);
}


#[test]
fn switch_s() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    let l;
    {
        #[derive(Clone)]
        struct SS {
            a: &'static str,
            b: &'static str,
            sw: Option<&'static str>
        }
        impl SS {
            fn new(a: &'static str, b: &'static str, sw: Option<&'static str>) -> SS {
                SS {
                    a: a,
                    b: b,
                    sw: sw
                }
            }
        }
        let sss = sodium_ctx.new_stream_sink();
        let sa = sss.stream().map(|s: &SS| s.a.clone());
        let sb = sss.stream().map(|s: &SS| s.b.clone());
        let csw_str =
            sss
                .stream()
                .map(
                    |s: &SS| s.sw.clone()
                )
                .filter_option()
                .hold("sa");
        let csw_deps = vec![sa.to_dep(), sb.to_dep()];
        let csw: Cell<Stream<&'static str>> = csw_str.map(
            lambda1(
                move |sw: &&'static str|
                    if *sw == "sa" { sa.clone() } else { sb.clone() },
                csw_deps
            )
        );
        let so = Cell::switch_s(&csw);
        let out = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        {
            let out = out.clone();
            l = so.listen(
                move |x: &&'static str|
                    out.lock().as_mut().unwrap().push(*x)
            );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        #[derive(Clone)]
        struct SS2 {
            s: StreamSink<i32>
        }
        impl SS2 {
            fn new(sodium_ctx: &SodiumCtx) -> SS2 {
                SS2 {
                    s: sodium_ctx.new_stream_sink()
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
            l = so.listen(
                move |c: &i32| out.lock().as_mut().unwrap().push(*c)
            );
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
        sodium_ctx.transaction(
            || {
                ss3.s.send(8);
                css.send(ss4.clone());
                ss4.s.send(2);
            }
        );
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
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sum_out = sodium_ctx.transaction(
            || {
                let sum = sodium_ctx.new_cell_loop();
                let sum_out = sa
                    .stream()
                    .snapshot(&sum.cell(), |x: &i32, y: &i32| *x + *y)
                    .hold(0);
                sum.loop_(&sum_out);
                sum_out
            }
        );
        let out = Arc::new(Mutex::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sum_out.listen(
                move |a: &i32|
                    out.lock().as_mut().unwrap().push(*a)
            );
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
