use sodium::Cell;
use sodium::CellLoop;
use sodium::CellSink;
use sodium::IsCell;
use sodium::IsStream;
use sodium::Operational;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamLoop;
use sodium::StreamSink;
use sodium::StreamWithSend;
use sodium::Transaction;
use tests::assert_memory_freed;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn map() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s: StreamSink<i32> = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.map(sodium_ctx, |a| *a + 1)
                .listen(
                    sodium_ctx,
                    move |a| {
                        (*out).borrow_mut().push(a.clone())
                    }
                );
        }
        s.send(sodium_ctx, &7);
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
        let s = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s.map_to(sodium_ctx, "fusebox")
                    .listen(
                        sodium_ctx,
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        s.send(sodium_ctx, &7);
        s.send(sodium_ctx, &9);
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
        let s1 = StreamSink::new(sodium_ctx);
        let s2 = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.or_else(sodium_ctx, &s1)
                    .listen(
                        sodium_ctx,
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        s1.send(sodium_ctx, &7);
        s2.send(sodium_ctx, &9);
        s1.send(sodium_ctx, &8);
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
        let s1 = StreamSink::new_with_coalescer(sodium_ctx, |l, r| *r);
        let s2 = StreamSink::new_with_coalescer(sodium_ctx, |l, r| *r);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s2.or_else(sodium_ctx, &s1)
                    .listen(
                        sodium_ctx,
                        move |a|
                            (*out).borrow_mut().push(*a)
                    );
        }
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s1.send(sodium_ctx, &7);
                s2.send(sodium_ctx, &60);
            }
        );
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s1.send(sodium_ctx, &9);
            }
        );
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s1.send(sodium_ctx, &7);
                s1.send(sodium_ctx, &60);
                s2.send(sodium_ctx, &8);
                s2.send(sodium_ctx, &90);
            }
        );
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s2.send(sodium_ctx, &8);
                s2.send(sodium_ctx, &90);
                s1.send(sodium_ctx, &7);
                s1.send(sodium_ctx, &60);
            }
        );
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s2.send(sodium_ctx, &8);
                s1.send(sodium_ctx, &7);
                s2.send(sodium_ctx, &90);
                s1.send(sodium_ctx, &60);
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
        let s = StreamSink::new_with_coalescer(sodium_ctx, |a, b| *a + *b);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s.listen(
                sodium_ctx,
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s.send(sodium_ctx, &2);
            }
        );
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                s.send(sodium_ctx, &8);
                s.send(sodium_ctx, &40);
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
        let s = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = s
                .filter(sodium_ctx, |a| *a < 10)
                .listen(
                    sodium_ctx,
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(sodium_ctx, &2);
        s.send(sodium_ctx, &16);
        s.send(sodium_ctx, &9);
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
        let s = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Stream::filter_option(sodium_ctx, &s)
                .listen(
                    sodium_ctx,
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(sodium_ctx, &Some("tomato"));
        s.send(sodium_ctx, &None);
        s.send(sodium_ctx, &Some("peach"));
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
        let sa = StreamSink::new(sodium_ctx);
        let sb =
            sa
                .map(sodium_ctx, |x| *x / 10)
                .filter(sodium_ctx, |x| *x != 0);
        let sc =
            sa
                .map(sodium_ctx, |x| *x % 10)
                .merge(sodium_ctx, &sb, |x, y| *x + *y);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                sodium_ctx,
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(sodium_ctx, &2);
        sa.send(sodium_ctx, &52);
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
        let sa = StreamSink::new(sodium_ctx);
        let sc = Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                let mut sb = StreamLoop::new(sodium_ctx);
                let sc_ =
                    sa
                        .map(sodium_ctx, |x| *x % 10)
                        .merge(sodium_ctx, &sb, |x, y| *x + *y);
                let sb_out =
                    sa
                        .map(sodium_ctx, |x| *x / 10)
                        .filter(sodium_ctx, |x| *x != 0);
                sb.loop_(sodium_ctx, sb_out);
                sc_
            }
        );
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sc.listen(
                sodium_ctx,
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(sodium_ctx, &2);
        sa.send(sodium_ctx, &52);
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
        let s = StreamSink::new(sodium_ctx);
        let pred = CellSink::new(sodium_ctx, true);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .gate(sodium_ctx, &pred)
                    .listen(
                        sodium_ctx,
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(sodium_ctx, &"H");
        pred.send(sodium_ctx, &false);
        s.send(sodium_ctx, &"O");
        pred.send(sodium_ctx, &true);
        s.send(sodium_ctx, &"I");
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
        let ea = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let sum = ea.collect(sodium_ctx, 0, |a,s| (*a + *s + 100, *a + *s));
        let l;
        {
            let out = out.clone();
            l =
                sum.listen(
                    sodium_ctx,
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        ea.send(sodium_ctx, &5);
        ea.send(sodium_ctx, &7);
        ea.send(sodium_ctx, &1);
        ea.send(sodium_ctx, &2);
        ea.send(sodium_ctx, &3);
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
        let ea = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let sum = ea.accum(sodium_ctx, 100, |a, s| *a + *s);
        let l;
        {
            let out = out.clone();
            l =
                sum.listen(
                    sodium_ctx,
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        ea.send(sodium_ctx, &5);
        ea.send(sodium_ctx, &7);
        ea.send(sodium_ctx, &1);
        ea.send(sodium_ctx, &2);
        ea.send(sodium_ctx, &3);
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
        let s = StreamSink::new(sodium_ctx);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s
                    .once(sodium_ctx)
                    .listen(
                        sodium_ctx,
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(sodium_ctx, &"A");
        s.send(sodium_ctx, &"B");
        s.send(sodium_ctx, &"C");
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
        let s = StreamSink::new(sodium_ctx);
        let c = s.hold(sodium_ctx, " ");
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                Operational
                    ::defer(sodium_ctx, &s)
                    .snapshot_to(sodium_ctx, &c)
                    .listen(
                        sodium_ctx,
                        move |a|
                            out.borrow_mut().push(*a)
                    );
        }
        s.send(sodium_ctx, &"C");
        s.send(sodium_ctx, &"B");
        s.send(sodium_ctx, &"A");
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
        let s = StreamSink::new(sodium_ctx);
        let c = s.hold(sodium_ctx, 0);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = Operational
                ::updates(sodium_ctx, &c)
                .listen(
                    sodium_ctx,
                    move |a|
                        out.borrow_mut().push(*a)
                );
        }
        s.send(sodium_ctx, &2);
        s.send(sodium_ctx, &9);
        l.unlisten();
        assert_eq!(vec![2, 9], *out.borrow());
    }
    assert_memory_freed(sodium_ctx);
}

fn hold_is_delayed() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let s = StreamSink::new(sodium_ctx);
        let h = s.hold(sodium_ctx, 0);
        let s_pair = s.snapshot(sodium_ctx, &h, |a, b| format!("{} {}", *a, *b));
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                s_pair
                    .listen(
                        sodium_ctx,
                        move |a|
                            out.borrow_mut().push(a.clone())
                    );
        }
        s.send(sodium_ctx, &2);
        s.send(sodium_ctx, &3);
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
        let ssc = StreamSink::new(sodium_ctx);
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let ca = Stream::filter_option(sodium_ctx, &ssc.map(sodium_ctx2, |s: &SC| s.a.clone())).hold(sodium_ctx, "A");
        let cb = Stream::filter_option(sodium_ctx, &ssc.map(sodium_ctx2, |s: &SC| s.b.clone())).hold(sodium_ctx, "a");
        let csw_str = Stream::filter_option(sodium_ctx, &ssc.map(sodium_ctx2, |s: &SC| s.sw.clone())).hold(sodium_ctx, "ca");
        let csw = csw_str.map(sodium_ctx, move |s| if *s == "ca" { ca.clone() } else { cb.clone() });
        let co = Cell::switch_c(sodium_ctx, &csw);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l =
                co.listen(
                    sodium_ctx,
                    move |c|
                        out.borrow_mut().push(*c)
                );
        }
        ssc.send(sodium_ctx, &SC::new(Some("B"), Some("b"), None));
        ssc.send(sodium_ctx, &SC::new(Some("C"), Some("c"), Some("cb")));
        ssc.send(sodium_ctx, &SC::new(Some("D"), Some("d"), None));
        ssc.send(sodium_ctx, &SC::new(Some("E"), Some("e"), Some("ca")));
        ssc.send(sodium_ctx, &SC::new(Some("F"), Some("f"), None));
        ssc.send(sodium_ctx, &SC::new(None, None, Some("cb")));
        ssc.send(sodium_ctx, &SC::new(None, None, Some("ca")));
        ssc.send(sodium_ctx, &SC::new(Some("G"), Some("g"), Some("cb")));
        ssc.send(sodium_ctx, &SC::new(Some("H"), Some("h"), Some("ca")));
        ssc.send(sodium_ctx, &SC::new(Some("I"), Some("i"), Some("ca")));
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
        let sa = sss.map(sodium_ctx, |s: &SS| s.a.clone());
        let sb = sss.map(sodium_ctx, |s: &SS| s.b.clone());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let csw_str =
            Stream
                ::filter_option(
                    sodium_ctx,
                    &sss.map(
                        sodium_ctx2,
                        |s| s.sw.clone()
                    )
                )
                .hold(sodium_ctx, "sa");
        let csw: Cell<Stream<&'static str>> = csw_str.map(
            sodium_ctx,
            move |sw|
                if *sw == "sa" { sa.clone() } else { sb.clone() }
        );
        let so = Cell::switch_s(sodium_ctx, &csw);
        let out = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let l;
        {
            let out = out.clone();
            l = so.listen(
                sodium_ctx,
                move |x|
                    out.borrow_mut().push(*x)
            );
        }
        sss.send(sodium_ctx, &SS::new("A", "a", None));
        sss.send(sodium_ctx, &SS::new("B", "b", None));
        sss.send(sodium_ctx, &SS::new("C", "c", Some("sb")));
        sss.send(sodium_ctx, &SS::new("D", "d", None));
        sss.send(sodium_ctx, &SS::new("E", "e", Some("sa")));
        sss.send(sodium_ctx, &SS::new("F", "f", None));
        sss.send(sodium_ctx, &SS::new("G", "g", Some("sb")));
        sss.send(sodium_ctx, &SS::new("H", "h", Some("sa")));
        sss.send(sodium_ctx, &SS::new("I", "i", Some("sa")));
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
        let css = CellSink::new(sodium_ctx, ss1.clone());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let so = Cell::switch_s(sodium_ctx, &css.map(sodium_ctx2, |b| b.s.clone()));
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = so.listen(
                sodium_ctx,
                move |c| out.borrow_mut().push(*c)
            );
        }
        ss1.s.send(sodium_ctx, &0);
        ss1.s.send(sodium_ctx, &1);
        ss1.s.send(sodium_ctx, &2);
        css.send(sodium_ctx, &ss2);
        ss1.s.send(sodium_ctx, &7);
        ss2.s.send(sodium_ctx, &3);
        ss2.s.send(sodium_ctx, &4);
        ss3.s.send(sodium_ctx, &2);
        css.send(sodium_ctx, &ss3);
        ss3.s.send(sodium_ctx, &5);
        ss3.s.send(sodium_ctx, &6);
        ss3.s.send(sodium_ctx, &7);
        Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                ss3.s.send(sodium_ctx, &8);
                css.send(sodium_ctx, &ss4);
                ss4.s.send(sodium_ctx, &2);
            }
        );
        ss4.s.send(sodium_ctx, &9);
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
        let sa = StreamSink::new(sodium_ctx);
        let sum_out = Transaction::run(
            sodium_ctx,
            |sodium_ctx| {
                let mut sum = CellLoop::new(sodium_ctx);
                let sum_out = sa
                    .snapshot(sodium_ctx, &sum, |x, y| *x + *y)
                    .hold(sodium_ctx, 0);
                sum.loop_(sodium_ctx, &sum_out);
                sum_out
            }
        );
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = sum_out.listen(
                sodium_ctx,
                move |a|
                    out.borrow_mut().push(*a)
            );
        }
        sa.send(sodium_ctx, &2);
        sa.send(sodium_ctx, &3);
        sa.send(sodium_ctx, &1);
        l.unlisten();
        assert_eq!(vec![0, 2, 5, 6], *out.borrow());
        assert_eq!(6, sum_out.sample(sodium_ctx));
    }
    assert_memory_freed(sodium_ctx);
}
