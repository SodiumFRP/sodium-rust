# Sodium
A Functional Reactive Programming (FRP) library for Rust

Avaliable on crates.io: https://crates.io/crates/sodium-rust

## Pitfalls

### No Global State

You must create a SodiumCtx for your application and keep passing it around in order to create sodium objects.

### Memory Management

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

## Special Features

### Pure Function Wrapping

You can do location mutation and create a SodiumCtx locally within the function, construct your full sodium graph locally within the function. Then execute that function externally as a pure function with no observable side effects, it will even be thread safe.

### Lock Free Concurrency

Since there is no global stage (no global variables) it is independent FRP graphs can be updated at the same time. When ```cargo tests``` is run on this library, all the tests actually run at the same time on different threads and the tests do no lock each other for shared access.

All sodium objects including the context ```SodiumCtx``` do not implement the ```Send``` / ```Sync``` traits whick means they can not be passed between threads. Instead you would have to use other idomatic rust techniques for passing information between threads, then use the ```::send``` of your ```StreamSink``` / ```CellSink``` of your target thread to keep the information flowing.
