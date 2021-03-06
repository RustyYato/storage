use core::{alloc::Layout, cell::RefCell, ptr::NonNull};

use crate::{
    Flush, FromPtr, MultiStorage, OffsetHandle, ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle,
    SharedResizableStorage, SharedStorage, Storage,
};

impl<S: Flush + ?Sized> Flush for RefCell<S> {
    fn try_flush(&mut self) -> bool { S::try_flush(self.get_mut()) }

    fn flush(&mut self) { S::flush(self.get_mut()) }
}

impl<S: Flush + ?Sized> SharedFlush for RefCell<S> {
    fn try_shared_flush(&self) -> bool { S::try_flush(&mut *self.borrow_mut()) }

    fn shared_flush(&self) { S::flush(&mut *self.borrow_mut()) }
}

unsafe impl<S: FromPtr + ?Sized> FromPtr for RefCell<S> {
    #[inline]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle {
        S::from_ptr_mut(&mut *self.borrow_mut(), ptr, layout)
    }

    #[inline]
    unsafe fn from_ptr_mut(&mut self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle {
        S::from_ptr_mut(self.get_mut(), ptr, layout)
    }
}

unsafe impl<S: OffsetHandle + ?Sized> OffsetHandle for RefCell<S> {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.get_mut().offset(handle, offset)
    }
}

unsafe impl<S: OffsetHandle + ?Sized> SharedOffsetHandle for RefCell<S> {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.borrow_mut().offset(handle, offset)
    }
}

impl<S: MultiStorage + ?Sized> MultiStorage for RefCell<S> {}

unsafe impl<S: Storage + ?Sized> Storage for RefCell<S> {
    type Handle = S::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { self.borrow().get(handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { self.get_mut().get_mut(handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        self.get_mut().deallocate_nonempty(handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().allocate(layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) { self.get_mut().deallocate(handle, layout) }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().allocate_zeroed(layout)
    }
}

unsafe impl<S: Storage + ?Sized> SharedGetMut for RefCell<S> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { self.borrow_mut().get_mut(handle) }
}

unsafe impl<S: ResizableStorage + ?Sized> ResizableStorage for RefCell<S> {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().grow(handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.get_mut().shrink(handle, old, new)
    }
}

unsafe impl<S: Storage + ?Sized> SharedStorage for RefCell<S> {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        self.borrow_mut().deallocate_nonempty(handle, layout)
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().allocate(layout)
    }

    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        self.borrow_mut().deallocate(handle, layout)
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().allocate_zeroed(layout)
    }
}

unsafe impl<S: ResizableStorage + ?Sized> SharedResizableStorage for RefCell<S> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().grow(handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.borrow_mut().shrink(handle, old, new)
    }
}
