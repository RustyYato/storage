use crate::{AllocErr, MemoryBlock, MultiStorage};
use core::alloc::Layout;

pub unsafe fn grow<S: MultiStorage>(
    mut storage: S,
    handle: S::Handle,
    old: Layout,
    new: Layout,
) -> Result<MemoryBlock<S::Handle>, AllocErr> {
    let memory_block = storage.allocate(new)?;
    let old_ptr = storage.get(handle);
    let new_ptr = storage.shared_get_mut(memory_block.handle);
    new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
    storage.deallocate(handle, old);
    Ok(memory_block)
}

pub unsafe fn grow_zeroed<S: MultiStorage>(
    mut storage: S,
    handle: S::Handle,
    old: Layout,
    new: Layout,
) -> Result<MemoryBlock<S::Handle>, AllocErr> {
    let memory_block = storage.allocate_zeroed(new)?;
    let old_ptr = storage.get(handle);
    let new_ptr = storage.shared_get_mut(memory_block.handle);
    new_ptr.as_ptr().copy_from_nonoverlapping(old_ptr.as_ptr(), old.size());
    storage.deallocate(handle, old);
    Ok(memory_block)
}

pub unsafe fn shrink<S: MultiStorage>(
    mut storage: S,
    handle: S::Handle,
    old: Layout,
    new: Layout,
) -> Result<MemoryBlock<S::Handle>, AllocErr> {
    let memory_block = storage.allocate_zeroed(new)?;
    let old_ptr = storage.get(handle);
    let new_ptr = storage.shared_get_mut(memory_block.handle);
    new_ptr
        .as_ptr()
        .copy_from_nonoverlapping(old_ptr.as_ptr(), memory_block.size);
    storage.deallocate(handle, old);
    Ok(memory_block)
}
