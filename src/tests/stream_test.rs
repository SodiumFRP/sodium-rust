use sodium::Cell;
use sodium::CellLoop;
use sodium::CellSink;
use sodium::IsCell;
use sodium::IsStream;
use sodium::IsStreamOption;
use sodium::Operational;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamLoop;
use sodium::StreamSink;
use sodium::Transaction;
use tests::assert_memory_freed;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn gc_crash_test() {
    let mut sodium_ctx = SodiumCtx::new();
    #[allow(unused_variables)]
    let sodium_ctx = &mut sodium_ctx;
}

#[test]
fn map() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s: StreamSink<i32> = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.map(|a: &i32| *a + 1)
                .listen(
                    move |a| {
                        (*out).borrow_mut().push(a.clone())
                    }
                );
        }
        s.send(&7);
        assert_eq!(vec![8], *(*out).borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn map_to() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s.map_to("fusebox")
                    .listen(
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        s.send(&7);
        s.send(&9);
        assert_eq!(vec!["fusebox", "fusebox"], *(*out).borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn merge_non_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink();
        let s2 = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.or_else(&s1)
                    .listen(
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        s1.send(&7);
        s2.send(&9);
        s1.send(&8);
        assert_eq!(vec![7, 9, 8], *(*out).borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn merge_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s1 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let s2 = sodium_ctx.new_stream_sink_with_coalescer(|_l, r| *r);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.or_else(&s1)
                    .listen(
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        sodium_ctx.run_transaction(
            || {
                s1.send(&7);
                s2.send(&60);
            }
        );
        sodium_ctx.run_transaction(
            || {
                s1.send(&9);
            }
        );
        sodium_ctx.run_transaction(
            || {
                s1.send(&7);
                s1.send(&60);
                s2.send(&8);
                s2.send(&90);
            }
        );
        sodium_ctx.run_transaction(
            || {
                s2.send(&8);
                s2.send(&90);
                s1.send(&7);
                s1.send(&60);
            }
        );
        sodium_ctx.run_transaction(
            || {
                s2.send(&8);
                s1.send(&7);
                s2.send(&90);
                s1.send(&60);
            }
        );
        assert_eq!(vec![60, 9, 90, 90, 90], *(*out).borrow());
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
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.listen(
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sodium_ctx.run_transaction(
            || {
                s.send(&2);
            }
        );
        sodium_ctx.run_transaction(
            || {
                s.send(&8);
                s.send(&40);
            }
        );
        assert_eq!(vec![2, 48], *out.borrow());
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
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .filter(|a: &u32| *a < 10)
                .listen(
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(&2);
        s.send(&16);
        s.send(&9);
        assert_eq!(vec![2, 9], *out.borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn filter_option() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.filter_option()
                .listen(
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(&Some("tomato"));
        s.send(&None);
        s.send(&Some("peach"));
        assert_eq!(vec!["tomato", "peach"], *out.borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn merge() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sb =
            sa
                .map(|x: &i32| *x / 10)
                .filter(|x: &i32| *x != 0);
        let sc =
            sa
                .map(|x: &i32| *x % 10)
                .merge(&sb, |x: &i32, y: &i32| *x + *y);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(&2);
        sa.send(&52);
        assert_eq!(vec![2, 7], *out.borrow());
        l.unlisten();
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let sc = sodium_ctx.run_transaction(
            || {
                let mut sb = sodium_ctx2.new_stream_loop();
                let sc_ =
                    sa
                        .map(|x: &i32| *x % 10)
                        .merge(&sb, |x: &i32, y: &i32| *x + *y);
                let sb_out =
                    sa
                        .map(|x: &i32| *x / 10)
                        .filter(|x: &i32| *x != 0);
                sb.loop_(sb_out);
                sc_
            }
        );
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(&2);
        sa.send(&52);
        l.unlisten();
        assert_eq!(vec![2, 7], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn gate() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let pred = sodium_ctx.new_cell_sink(true);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .gate(&pred)
                    .listen(
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(&"H");
        pred.send(&false);
        s.send(&"O");
        pred.send(&true);
        s.send(&"I");
        l.unlisten();

        assert_eq!(vec!["H", "I"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn collect() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let sum = ea.collect(0, |a,s| (*a + *s + 100, *a + *s));
        let l;
        {
            let out = out.clone();
            l =
                sum.listen(
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        ea.send(&5);
        ea.send(&7);
        ea.send(&1);
        ea.send(&2);
        ea.send(&3);
        l.unlisten();
        assert_eq!(vec![105, 112, 113, 115, 118], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn accum() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let ea = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let sum = ea.accum(100, |a, s| *a + *s);
        let l;
        {
            let out = out.clone();
            l =
                sum.listen(
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        ea.send(&5);
        ea.send(&7);
        ea.send(&1);
        ea.send(&2);
        ea.send(&3);
        l.unlisten();
        assert_eq!(vec![100, 105, 112, 113, 115, 118], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn once() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .once()
                    .listen(
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(&"A");
        s.send(&"B");
        s.send(&"C");
        l.unlisten();
        assert_eq!(vec!["A"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn defer() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.hold(" ");
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                Operational
                    ::defer(&s)
                    .snapshot_to(&c)
                    .listen(
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(&"C");
        s.send(&"B");
        s.send(&"A");
        l.unlisten();
        assert_eq!(vec!["C", "B", "A"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn hold() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let c = s.hold(0);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Operational
                ::updates(&c)
                .listen(
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(&2);
        s.send(&9);
        l.unlisten();
        assert_eq!(vec![2, 9], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn hold_is_delayed() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = sodium_ctx.new_stream_sink();
        let h = s.hold(0);
        let s_pair = s.snapshot(&h, |a: &i32, b: &i32| format!("{} {}", *a, *b));
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s_pair
                    .listen(
                        move |a|
                            out.borrow_mut().push(a.clone())
                    );
        }
        s.send(&2);
        s.send(&3);
        l.unlisten();
        assert_eq!(vec![String::from("2 0"), String::from("3 2")], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_c() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
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
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let ca = ssc.map(|s: &SC| s.a.clone()).filter_option().hold("A");
        let cb = ssc.map(|s: &SC| s.b.clone()).filter_option().hold("a");
        let csw_str = ssc.map(|s: &SC| s.sw.clone()).filter_option().hold("ca");
        let csw = csw_str.map(move |s: &&'static str| if *s == "ca" { ca.clone() } else { cb.clone() });
        let co = Cell::switch_c(&csw);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                co.listen(
                    move |c|
                        out.borrow_mut().push(*c)
                );
        }
        ssc.send(&SC::new(Some("B"), Some("b"), None));
        ssc.send(&SC::new(Some("C"), Some("c"), Some("cb")));
        ssc.send(&SC::new(Some("D"), Some("d"), None));
        ssc.send(&SC::new(Some("E"), Some("e"), Some("ca")));
        ssc.send(&SC::new(Some("F"), Some("f"), None));
        ssc.send(&SC::new(None, None, Some("cb")));
        ssc.send(&SC::new(None, None, Some("ca")));
        ssc.send(&SC::new(Some("G"), Some("g"), Some("cb")));
        ssc.send(&SC::new(Some("H"), Some("h"), Some("ca")));
        ssc.send(&SC::new(Some("I"), Some("i"), Some("ca")));
        l.unlisten();
        assert_eq!(vec!["A", "B", "c", "d", "E", "F", "f", "F", "g", "H", "I"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_s() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
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
        let sss = StreamSink::new(sodium_ctx);
        let sa = sss.map(|s: &SS| s.a.clone());
        let sb = sss.map(|s: &SS| s.b.clone());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let csw_str =
            sss
                .map(
                    |s: &SS| s.sw.clone()
                )
                .filter_option()
                .hold("sa");
        let csw: Cell<Stream<&'static str>> = csw_str.map(
            move |sw: &&'static str|
                if *sw == "sa" { sa.clone() } else { sb.clone() }
        );
        let so = Cell::switch_s(&csw);
        let out = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let l;
        {
            let out = out.clone();
            l = so.listen(
                move |x|
                    out.borrow_mut().push(*x)
            );
        }
        sss.send(&SS::new("A", "a", None));
        sss.send(&SS::new("B", "b", None));
        sss.send(&SS::new("C", "c", Some("sb")));
        sss.send(&SS::new("D", "d", None));
        sss.send(&SS::new("E", "e", Some("sa")));
        sss.send(&SS::new("F", "f", None));
        sss.send(&SS::new("G", "g", Some("sb")));
        sss.send(&SS::new("H", "h", Some("sa")));
        sss.send(&SS::new("I", "i", Some("sa")));
        l.unlisten();
        assert_eq!(vec!["A", "B", "C", "d", "e", "F", "G", "h", "I"], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn switch_s_simultaneous() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        #[derive(Clone)]
        struct SS2 {
            s: StreamSink<i32>
        }
        impl SS2 {
            fn new(sodium_ctx: &mut SodiumCtx) -> SS2 {
                SS2 {
                    s: StreamSink::new(sodium_ctx)
                }
            }
        }
        let ss1 = SS2::new(sodium_ctx);
        let ss2 = SS2::new(sodium_ctx);
        let ss3 = SS2::new(sodium_ctx);
        let ss4 = SS2::new(sodium_ctx);
        let css = sodium_ctx.new_cell_sink(ss1.clone());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let so = Cell::switch_s(&css.map(|b: &SS2| b.s.clone()));
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = so.listen(
                move |c| out.borrow_mut().push(*c)
            );
        }
        ss1.s.send(&0);
        ss1.s.send(&1);
        ss1.s.send(&2);
        css.send(&ss2);
        ss1.s.send(&7);
        ss2.s.send(&3);
        ss2.s.send(&4);
        ss3.s.send(&2);
        css.send(&ss3);
        ss3.s.send(&5);
        ss3.s.send(&6);
        ss3.s.send(&7);
        sodium_ctx.run_transaction(
            || {
                ss3.s.send(&8);
                css.send(&ss4);
                ss4.s.send(&2);
            }
        );
        ss4.s.send(&9);
        l.unlisten();
        assert_eq!(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

#[test]
fn loop_cell() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let sa = sodium_ctx.new_stream_sink();
        let sum_out = Transaction::run(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx| {
                let mut sum = sodium_ctx.new_cell_loop();
                let sum_out = sa
                    .snapshot(&sum, |x: &i32, y: &i32| *x + *y)
                    .hold(0);
                sum.loop_(&sum_out);
                sum_out
            }
        );
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sum_out.listen(
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(&2);
        sa.send(&3);
        sa.send(&1);
        l.unlisten();
        assert_eq!(vec![0, 2, 5, 6], *out.borrow());
        assert_eq!(6, sum_out.sample());
    }
    assert_memory_freed(sodium_ctx);
}
