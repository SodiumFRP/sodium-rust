use sodium::Cell;
use sodium::IsCell;
use sodium::SodiumCtx;
use tests::assert_memory_freed;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn constant_cell() {
    let mut sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &mut sodium_ctx;
    {
        let c = Cell::new(sodium_ctx, 12);
        let out = Rc::new(RefCell::new(Vec::new()));
        let l;
        {
            let out = out.clone();
            l = c.listen(
                sodium_ctx,
                move |a|
                    (*out).borrow_mut().push(a.clone())
            );
        }
        assert_eq!(vec![12], *(*out).borrow());
        l.unlisten();
    }
    //assert_memory_freed(sodium_ctx);
}
