use core::{alloc::Layout, ptr::NonNull};

use crate::{AllocErr, MemoryBlock, NonEmptyLayout, NonEmptyMemoryBlock};

pub unsafe trait Handle: Copy {
    /// # Safety
    ///
    /// align must be a power of two
    unsafe fn dangling(align: usize) -> Self;
}

pub unsafe trait PointerHandle: Copy + Handle {
    unsafe fn get(self) -> NonNull<u8>;

    unsafe fn get_mut(self) -> NonNull<u8>;
}

pub unsafe trait FromPtr: Storage {
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle;
}

pub unsafe trait SharedGetMut: Storage {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8>;
}

pub trait MultiStorage: SharedGetMut {}

pub unsafe trait Storage {
    type Handle: Handle;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8>;

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8>;

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout);

    fn allocate(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        match NonEmptyLayout::new(layout) {
            Some(layout) => self.allocate_nonempty(layout).map(Into::into),
            None => Ok(MemoryBlock {
                handle: unsafe { Handle::dangling(layout.align()) },
                size: 0,
            }),
        }
    }

    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) {
        if let Some(layout) = NonEmptyLayout::new(layout) {
            self.deallocate_nonempty(handle, layout)
        }
    }

    fn allocate_nonempty_zeroed(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.allocate_nonempty(layout)?;

        unsafe {
            let ptr = self.get_mut(memory_block.handle);
            ptr.as_ptr().write_bytes(0, memory_block.size.get());
        }

        Ok(memory_block)
    }

    fn allocate_zeroed(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        match NonEmptyLayout::new(layout) {
            Some(layout) => self.allocate_nonempty_zeroed(layout).map(Into::into),
            None => Ok(MemoryBlock {
                handle: unsafe { Handle::dangling(layout.align()) },
                size: 0,
            }),
        }
    }
}

pub unsafe trait ResizableStorage: Storage {
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;
}

pub unsafe trait SharedStorage: SharedGetMut {
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout);

    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        match NonEmptyLayout::new(layout) {
            Some(layout) => self.shared_allocate_nonempty(layout).map(Into::into),
            None => Ok(MemoryBlock {
                handle: unsafe { Handle::dangling(layout.align()) },
                size: 0,
            }),
        }
    }

    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        if let Some(layout) = NonEmptyLayout::new(layout) {
            self.shared_deallocate_nonempty(handle, layout)
        }
    }

    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.shared_allocate_nonempty(layout)?;

        unsafe {
            let ptr = self.shared_get_mut(memory_block.handle);
            ptr.as_ptr().write_bytes(0, memory_block.size.get());
        }

        Ok(memory_block)
    }

    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        match NonEmptyLayout::new(layout) {
            Some(layout) => self.shared_allocate_nonempty_zeroed(layout).map(Into::into),
            None => Ok(MemoryBlock {
                handle: unsafe { Handle::dangling(layout.align()) },
                size: 0,
            }),
        }
    }
}

pub unsafe trait SharedResizableStorage: SharedStorage + ResizableStorage {
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;

    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr>;
}
