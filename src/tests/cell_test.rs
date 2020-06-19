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

/*
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
