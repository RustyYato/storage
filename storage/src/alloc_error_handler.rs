use core::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, Ordering::SeqCst},
};

static ALLOC_ERROR_HANDLER: AtomicPtr<()> = AtomicPtr::new(default_alloc_error_handler as Handler as *mut ());
type Handler = fn(Layout) -> !;

pub fn set_alloc_error_handler(handler: Handler) { ALLOC_ERROR_HANDLER.store(handler as *mut (), SeqCst) }

#[cold]
pub fn handle_alloc_error(layout: Layout) -> ! {
    let handler = unsafe { core::mem::transmute::<*mut (), Handler>(ALLOC_ERROR_HANDLER.load(SeqCst)) };
    handler(layout)
}

fn default_alloc_error_handler(_: Layout) -> ! { core::intrinsics::abort() }
