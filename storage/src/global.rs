use core::{
    ptr::NonNull,
    sync::atomic::{
        AtomicU8,
        Ordering::{Relaxed, SeqCst},
    },
};

use crate::{
    affix::{OffsetHandle, SharedOffsetHandle},
    AllocErr, FromPtr, MultiStorage, NonEmptyLayout, ResizableStorage, SharedGetMut, SharedResizableStorage,
    SharedStorage, Storage,
};

pub trait GlobalStorage: SharedResizableStorage + Send + Sync + 'static {}
impl<T: ?Sized + SharedResizableStorage + Send + Sync + 'static> GlobalStorage for T {}

#[derive(Default, Debug, Clone, Copy)]
pub struct Global;

pub type GlobalStorageImp = &'static dyn GlobalStorage<Handle = NonNull<u8>>;

static mut GLOBAL: GlobalStorageImp = &crate::no_op::NoOpStorage;
static INITIALIZER_STATE: AtomicU8 = AtomicU8::new(UNINIT);

const UNINIT: u8 = 0;
const WRITING: u8 = 1;
const INIT: u8 = 2;

pub fn set_global_storage(global: GlobalStorageImp) -> bool {
    if INITIALIZER_STATE.load(Relaxed) != UNINIT
        || INITIALIZER_STATE
            .compare_exchange(UNINIT, WRITING, SeqCst, Relaxed)
            .is_err()
    {
        return false
    }

    unsafe {
        GLOBAL = global;
    }

    INITIALIZER_STATE.store(INIT, SeqCst);

    true
}

#[inline]
fn global() -> GlobalStorageImp {
    if INITIALIZER_STATE.load(Relaxed) == INIT {
        unsafe { GLOBAL }
    } else {
        &crate::no_op::NoOpStorage
    }
}

unsafe impl FromPtr for Global {
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { ptr }
}

unsafe impl SharedGetMut for Global {
    #[inline]
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { handle }
}

impl MultiStorage for Global {}

unsafe impl OffsetHandle for Global {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        NonNull::new_unchecked(handle.as_ptr().offset(offset))
    }
}

unsafe impl SharedOffsetHandle for Global {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        NonNull::new_unchecked(handle.as_ptr().offset(offset))
        // Global.offset(handle, offset)
    }
}

unsafe impl Storage for Global {
    type Handle = NonNull<u8>;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { handle }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { handle }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        global().deallocate_nonempty(handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().allocate(layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        global().deallocate(handle, layout)
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_zeroed(layout)
    }
}

unsafe impl ResizableStorage for Global {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().grow(handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().shrink(handle, old, new)
    }
}

unsafe impl SharedStorage for Global {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        global().deallocate_nonempty(handle, layout)
    }

    #[inline]
    fn shared_allocate(&self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().allocate(layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
        global().deallocate(handle, layout)
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn shared_allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().allocate_zeroed(layout)
    }
}

unsafe impl SharedResizableStorage for Global {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().grow(handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        global().shrink(handle, old, new)
    }
}
