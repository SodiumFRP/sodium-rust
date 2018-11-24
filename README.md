# Sodium
A Functional Reactive Programming (FRP) library for Rust

Avaliable on crates.io: https://crates.io/crates/sodium-rust

# Pitfalls

To allow for sodium objects (or structs containing them) to be sent through ```StreamSink``` / ```CellSink```, ```Trace``` and ```Finalize``` traits must be implemented for them. If you have a struct that you know will not contain any sodium objects, then you can wrap it in ```NoGc``` to avoid having to implement those ```Trace``` and ```Finalize``` traits for it.

E.g.
```
        #[derive(Clone)]
        struct MyStruct {
            a: u32,
            b: u32
        }
        impl MyStruct {
            fn new(a: u32, b: u32) -> MyStruct {
                MyStruct { a, b }
            }
        }
        let s2: StreamSink<NoGc<MyStruct>> = sodium_ctx.new_stream_sink();
        s2.send(&NoGc::new(MyStruct::new(1,2)));
```

A ```NoGc<A>``` can be turned into a ```&A``` using the derefering operator ```*```, E.g. ```*my_value```.

If you are however, passing a struct that does reference sodium objects, then you must implement the ```Trace``` and ```Finalize``` traces for it.

E.g.
```
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
        impl Finalize for SS2 {
            fn finalize(&mut self) {}
        }
        impl Trace for SS2 {
            fn trace(&self, f: &mut FnMut(&GcDep)) {
                self.s.trace(f)
            }
        }
```
