use core::{alloc::Layout, ptr::NonNull};

mod choose;

pub use choose::{AndC, Choose, MaxAlign, MaxSize, MinAlign, MinSize, NotC, OrC};

use crate::{
    FromPtr, MultiStorage, PointerHandle, ResizableStorage, SharedGetMut, SharedResizableStorage, SharedStorage,
    Storage,
};

pub struct Picker<F, A, B> {
    pub choose: F,
    pub left: A,
    pub right: B,
}

unsafe impl<F: Choose, A: Storage, B: Storage<Handle = A::Handle>> SharedGetMut for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    #[inline]
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { handle.get_mut() }
}

unsafe impl<F: Choose, A: FromPtr, B: FromPtr<Handle = A::Handle>> FromPtr for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    unsafe fn from_ptr(&self, ptr: core::ptr::NonNull<u8>, layout: Layout) -> Self::Handle {
        if self.choose.choose(layout) {
            self.left.from_ptr(ptr, layout)
        } else {
            self.right.from_ptr(ptr, layout)
        }
    }
}

impl<F: Choose, A: MultiStorage, B: MultiStorage<Handle = A::Handle>> MultiStorage for Picker<F, A, B> where
    A::Handle: PointerHandle
{
}

unsafe impl<F: Choose, A: Storage, B: Storage<Handle = A::Handle>> Storage for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    type Handle = A::Handle;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { handle.get() }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { handle.get_mut() }

    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout.into()) {
            self.left.allocate_nonempty(layout)
        } else {
            self.right.allocate_nonempty(layout)
        }
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        if self.choose.choose(layout.into()) {
            self.left.deallocate_nonempty(handle, layout)
        } else {
            self.right.deallocate_nonempty(handle, layout)
        }
    }

    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout) {
            self.left.allocate(layout)
        } else {
            self.right.allocate(layout)
        }
    }

    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) {
        if self.choose.choose(layout) {
            self.left.deallocate(handle, layout)
        } else {
            self.right.deallocate(handle, layout)
        }
    }

    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout.into()) {
            self.left.allocate_nonempty_zeroed(layout)
        } else {
            self.right.allocate_nonempty_zeroed(layout)
        }
    }

    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout) {
            self.left.allocate_zeroed(layout)
        } else {
            self.right.allocate_zeroed(layout)
        }
    }
}

unsafe impl<F: Choose, A: ResizableStorage, B: ResizableStorage<Handle = A::Handle>> ResizableStorage
    for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.grow(handle, old, new),
            (false, false) => self.right.grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.allocate(new)
                } else {
                    self.right.allocate(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
                if o {
                    self.right.deallocate(handle, old)
                } else {
                    self.left.deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.grow(handle, old, new),
            (false, false) => self.right.grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.allocate_zeroed(new)
                } else {
                    self.right.allocate_zeroed(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
                if o {
                    self.right.deallocate(handle, old)
                } else {
                    self.left.deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.grow(handle, old, new),
            (false, false) => self.right.grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.allocate(new)
                } else {
                    self.right.allocate(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr
                    .as_ptr()
                    .copy_from_nonoverlapping(old_ptr.as_ptr(), memory_block.size);
                if o {
                    self.right.deallocate(handle, old)
                } else {
                    self.left.deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }
}

unsafe impl<F: Choose, A: SharedStorage, B: SharedStorage<Handle = A::Handle>> SharedStorage for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout.into()) {
            self.left.shared_allocate_nonempty(layout)
        } else {
            self.right.shared_allocate_nonempty(layout)
        }
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        if self.choose.choose(layout.into()) {
            self.left.shared_deallocate_nonempty(handle, layout)
        } else {
            self.right.shared_deallocate_nonempty(handle, layout)
        }
    }

    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout) {
            self.left.shared_allocate(layout)
        } else {
            self.right.shared_allocate(layout)
        }
    }

    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        if self.choose.choose(layout) {
            self.left.shared_deallocate(handle, layout)
        } else {
            self.right.shared_deallocate(handle, layout)
        }
    }

    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout.into()) {
            self.left.shared_allocate_nonempty_zeroed(layout)
        } else {
            self.right.shared_allocate_nonempty_zeroed(layout)
        }
    }

    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        if self.choose.choose(layout) {
            self.left.shared_allocate_zeroed(layout)
        } else {
            self.right.shared_allocate_zeroed(layout)
        }
    }
}

unsafe impl<F: Choose, A: SharedResizableStorage, B: SharedResizableStorage<Handle = A::Handle>> SharedResizableStorage
    for Picker<F, A, B>
where
    A::Handle: PointerHandle,
{
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.shared_grow(handle, old, new),
            (false, false) => self.right.shared_grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.shared_allocate(new)
                } else {
                    self.right.shared_allocate(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
                if o {
                    self.right.shared_deallocate(handle, old)
                } else {
                    self.left.shared_deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }

    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.shared_grow(handle, old, new),
            (false, false) => self.right.shared_grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.shared_allocate_zeroed(new)
                } else {
                    self.right.shared_allocate_zeroed(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
                if o {
                    self.right.shared_deallocate(handle, old)
                } else {
                    self.left.shared_deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }

    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match (self.choose.choose(old), self.choose.choose(new)) {
            (true, true) => self.left.shared_grow(handle, old, new),
            (false, false) => self.right.shared_grow(handle, old, new),
            (o, _) => {
                let memory_block = if o {
                    self.left.shared_allocate(new)
                } else {
                    self.right.shared_allocate(new)
                };
                let memory_block = memory_block?;
                let old_ptr = handle.get();
                let new_ptr = memory_block.handle.get_mut();
                new_ptr
                    .as_ptr()
                    .copy_from_nonoverlapping(old_ptr.as_ptr(), memory_block.size);
                if o {
                    self.right.shared_deallocate(handle, old)
                } else {
                    self.left.shared_deallocate(handle, old)
                }
                Ok(memory_block)
            }
        }
    }
}
