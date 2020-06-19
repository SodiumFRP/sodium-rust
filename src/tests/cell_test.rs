use crate::Cell;
use crate::SodiumCtx;
use crate::tests::assert_memory_freed;
use crate::tests::init;

use std::sync::Arc;
use std::sync::Mutex;

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
            l = c.listen(
                move |a: &i32|
                    out.lock().as_mut().unwrap().push(a.clone())
            );
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

/*
  'should test values'(done) {
    const c = new CellSink<number>(9),
      out: number[] = [],
      kill = c.listen(a => {
        out.push(a);
        if (out.length === 3) {
          done();
        }
      });

    c.send(2);
    c.send(7);
    kill();

    expect([9, 2, 7]).to.deep.equal(out);
  };
*/

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
            l = c.cell().map(|a: &i32| format!("{}", a)).listen(
                move |a: &String|
                    out.lock().as_mut().unwrap().push(a.clone())
            );
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
            r = c.map(move |c2:&Cell<i32>| c2.lift2(&s.cell(), |v1:&i32, v2:&i32| *v1 + *v2));
        }
        {
            let out = out.clone();
            l = Cell::switch_c(&r).listen(move |a:&i32| {
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

/*
  "should throw an error on mapCLateListen"() {
    const c = new CellSink<number>(6),
      out: string[] = [],
      cm = c.map(a => "" + a);

    try {
      c.send(2);
      const kill = cm.listen(a => out.push(a));
      c.send(8);
      kill();
    } catch (e) {

      expect(e.message).to.equal('send() was invoked before listeners were registered');
    }

    //assertEquals(["2", "8"], out);
  };

  "should test apply"(done) {
    const cf = new CellSink<(a: number) => string>(a => "1 " + a),
      ca = new CellSink<number>(5),
      out: string[] = [],
      kill = Cell.apply(cf, ca).listen(a => {
        out.push(a);
        if (out.length === 3) {
          done();
        }
      });

    cf.send(a => "12 " + a);
    ca.send(6);
    kill();

    expect(["1 5", "12 5", "12 6"]).to.deep.equal(out);
  };

  "should test lift"(done) {
    const a = new CellSink<number>(1),
      b = new CellSink<number>(5),
      out: string[] = [],
      kill = a.lift(b, (aa, bb) => aa + " " + bb)
        .listen(a => {
          out.push(a);
          if (out.length === 3) {
            done();
          }
        });
    a.send(12);
    b.send(6);
    kill();

    expect(["1 5", "12 5", "12 6"]).to.deep.equal(out);
  };

  "should test liftGlitch"(done) {
    const a = new CellSink(1),
      a3 = a.map(x => x * 3),
      a5 = a.map(x => x * 5),
      b = a3.lift(a5, (x, y) => x + " " + y),
      out: string[] = [],
      kill = b.listen(x => {
        out.push(x);
        if (out.length === 2) {
          done();
        }

      });
    a.send(2);
    kill();

    expect(["3 5", "6 10"]).to.deep.equal(out);
  };

  "should test liftFromSimultaneous"(done) {
    const t = Transaction.run(() => {
      const b1 = new CellSink(3),
        b2 = new CellSink(5);
      b2.send(7);
      return new Tuple2(b1, b2);
    });

    const b1 = t.a,
      b2 = t.b,
      out: number[] = [],
      kill = b1.lift(b2, (x, y) => x + y)
        .listen(a => {
          out.push(a);
          done();
        });
    kill();

    expect([10]).to.deep.equal(out);
  };

}*/
