use sodium::CellSink;
use sodium::IsCell;
use sodium::IsStream;
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

/*
    'should test defer()' (done) {
      const s = new StreamSink<string>(),
        c = s.hold(" "),
        out: string[] = [],
        kill = Operational.defer(s).snapshot1(c)
          .listen(a => {
            out.push(a);
            if(out.length === 3) {
              done();
            }
          });

      s.send("C");
      s.send("B");
      s.send("A");
      kill();

      expect(["C", "B", "A"]).to.deep.equal(out);
    };

    'should test hold()' (done) {
      const s = new StreamSink<number>(),
        c = s.hold(0),
        out: number[] = [],
        kill = Operational.updates(c)
          .listen(a => {
            out.push(a);
            if(out.length === 2) {
              done();
            }
          });

      s.send(2);
      s.send(9);
      kill();

      expect([2, 9]).to.deep.equal(out);
    };

    'should do holdIsDelayed' (done) {
      const s = new StreamSink<number>(),
        h = s.hold(0),
        sPair = s.snapshot(h, (a, b) => a + " " + b),
        out: string[] = [],
        kill = sPair.listen(a => {
          out.push(a);
          if(out.length === 2) {
            done();
          }
        });

      s.send(2);
      s.send(3);
      kill();

      expect(["2 0", "3 2"]).to.deep.equal(out);
    };

    'should test switchC()' (done) {
      class SC {
        constructor(a: string, b: string, sw: string) {
          this.a = a;
          this.b = b;
          this.sw = sw;
        }

        a: string;
        b: string;
        sw: string;
      }

      const ssc = new StreamSink<SC>(),
        // Split each field out of SC so we can update multiple cells in a
        // single transaction.
        ca = ssc.map(s => s.a).filterNotNull().hold("A"),
        cb = ssc.map(s => s.b).filterNotNull().hold("a"),
        csw_str = ssc.map(s => s.sw).filterNotNull().hold("ca"),
        // ****
        // NOTE! Because this lambda contains references to Sodium objects, we
        // must declare them explicitly using lambda1() so that Sodium knows
        // about the dependency, otherwise it can't manage the memory.
        // ****
        csw = csw_str.map(lambda1(s => s == "ca" ? ca : cb, [ca, cb])),
        co = Cell.switchC(csw),
        out: string[] = [],
        kill = co.listen(c => {
          out.push(c);
          if(out.length === 11) {
            done();
          }
        });

      ssc.send(new SC("B", "b", null));
      ssc.send(new SC("C", "c", "cb"));
      ssc.send(new SC("D", "d", null));
      ssc.send(new SC("E", "e", "ca"));
      ssc.send(new SC("F", "f", null));
      ssc.send(new SC(null, null, "cb"));
      ssc.send(new SC(null, null, "ca"));
      ssc.send(new SC("G", "g", "cb"));
      ssc.send(new SC("H", "h", "ca"));
      ssc.send(new SC("I", "i", "ca"));
      kill();

      expect(["A", "B", "c", "d", "E", "F", "f", "F", "g", "H", "I"]).to.deep.equal(out);

    };

    'should test switchS()' (done) {
      class SS {
        constructor(a: string, b: string, sw: string) {
          this.a = a;
          this.b = b;
          this.sw = sw;
        }

        a: string;
        b: string;
        sw: string;
      }

      const sss = new StreamSink<SS>(),
        sa = sss.map(s => s.a),
        sb = sss.map(s => s.b),
        csw_str = sss.map(s => s.sw).filterNotNull().hold("sa"),
        // ****
        // NOTE! Because this lambda contains references to Sodium objects, we
        // must declare them explicitly using lambda1() so that Sodium knows
        // about the dependency, otherwise it can't manage the memory.
        // ****
        csw = csw_str.map(lambda1(sw => sw == "sa" ? sa : sb, [sa, sb])),
        so = Cell.switchS(csw),
        out: string[] = [],
        kill = so.listen(x => {
          out.push(x);
          if(out.length === 9) {
            done();
          }
        });

      sss.send(new SS("A", "a", null));
      sss.send(new SS("B", "b", null));
      sss.send(new SS("C", "c", "sb"));
      sss.send(new SS("D", "d", null));
      sss.send(new SS("E", "e", "sa"));
      sss.send(new SS("F", "f", null));
      sss.send(new SS("G", "g", "sb"));
      sss.send(new SS("H", "h", "sa"));
      sss.send(new SS("I", "i", "sa"));
      kill();

      expect(["A", "B", "C", "d", "e", "F", "G", "h", "I"]).to.deep.equal(out);
    };

    'should do switchSSimultaneous' (done) {
      class SS2 {
        s: StreamSink<number> = new StreamSink<number>();
      }

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
        so = Cell.switchS(css.map(lambda1((b: SS2) => b.s, [ss1.s, ss2.s, ss3.s, ss4.s]))),
        out: number[] = [],
        kill = so.listen(c => {
          out.push(c);
          if(out.length === 10) {
            done();
          }
        });

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

      expect([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).to.deep.equal(out);
    };

    'should test loopCell' (done)  {
      const sa = new StreamSink<number>(),
        sum_out = Transaction.run(() => {
          const sum = new CellLoop<number>(),
            sum_out_ = sa.snapshot(sum, (x, y) => x + y).hold(0);
          sum.loop(sum_out_);
          return sum_out_;
        }),
        out: number[] = [],
        kill = sum_out.listen(a => {
          out.push(a);
          if(out.length === 4) {
            done();
          }
        });

      sa.send(2);
      sa.send(3);
      sa.send(1);
      kill();

      expect([0, 2, 5, 6]).to.deep.equal(out);
      expect(6).to.equal(sum_out.sample());
    };
  }
*/