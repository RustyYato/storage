#![no_std]
#![feature(core_intrinsics, ptr_metadata, unsize, layout_for_ptr)]
#![deny(clippy::pedantic, clippy::perf)]
#![warn(clippy::nursery)]
#![allow(
    clippy::declare_interior_mutable_const,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc
)]
#![allow(clippy::missing_safety_doc)]

#[doc(hidden)]
#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod macros;

#[cfg(test)]
extern crate std;

mod core_traits;

mod non_empty_layout;

pub mod affix;
mod bump;
mod global;
mod global_as_ptr;
mod imp;
mod multi;
mod no_op;
mod picker;
mod single;

pub mod defaults;

mod alloc_error_handler;

pub mod boxed;
pub mod rc;

mod scope_guard;

pub use core_traits::{
    FromPtr, Handle, MultiStorage, PointerHandle, ResizableStorage, SharedGetMut, SharedResizableStorage,
    SharedStorage, Storage,
};

pub use alloc_error_handler::{handle_alloc_error, set_alloc_error_handler};

pub use affix::{AffixHandle, AffixStorage};
pub use bump::{BumpHandle, BumpStorage};
pub use global::{set_global_storage, Global, GlobalStorage};
pub use global_as_ptr::GlobalAsPtrStorage;
pub use multi::{MultiHandle, MultiStackStorage};
pub use picker::{AndC, Choose, MaxAlign, MaxSize, MinAlign, MinSize, NotC, OrC, Picker};
pub use single::{OffsetSingleStackStorage, SingleStackStorage};

use core::{alloc::Layout, num::NonZeroUsize, ptr::NonNull};
pub use non_empty_layout::NonEmptyLayout;

#[derive(Debug)]
pub struct AllocErr(pub Layout);

impl AllocErr {
    #[inline]
    pub fn handle<T>(self) -> T { handle_alloc_error(self.0) }
}

unsafe impl Handle for () {
    unsafe fn dangling(_: usize) {}
}

unsafe impl Handle for NonNull<u8> {
    #[inline]
    unsafe fn dangling(align: usize) -> Self { Self::new_unchecked(align as *mut u8) }
}

unsafe impl PointerHandle for NonNull<u8> {
    #[inline]
    unsafe fn get(self) -> NonNull<u8> { self }

    #[inline]
    unsafe fn get_mut(self) -> NonNull<u8> { self }
}

pub struct NonEmptyMemoryBlock<Handle> {
    pub handle: Handle,
    pub size: NonZeroUsize,
}

pub struct MemoryBlock<Handle> {
    pub handle: Handle,
    pub size: usize,
}

impl<Handle> From<NonEmptyMemoryBlock<Handle>> for MemoryBlock<Handle> {
    fn from(memory: NonEmptyMemoryBlock<Handle>) -> Self {
        Self {
            handle: memory.handle,
            size: memory.size.get(),
        }
    }
}

#[test]
fn test() {
    let mut multi = MultiStackStorage::<[u8; 4096]>::new();
    let mut multi = unsafe { core::pin::Pin::new_unchecked(&mut multi) };

    let block = multi.allocate(Layout::new::<usize>()).unwrap();

    let handle = block.handle;

    unsafe {
        let ptr = Storage::get_mut(&mut multi, handle);
        let ptr = ptr.cast::<usize>().as_ptr();
        ptr.write(0xdead_beef);

        let new_block = multi.allocate(Layout::new::<[usize; 8]>()).unwrap();

        let ptr = Storage::get_mut(&mut multi, handle);
        let ptr = ptr.cast::<usize>().as_ptr();
        assert_eq!(ptr.read(), 0xdead_beef);

        multi.deallocate(new_block.handle, Layout::new::<[usize; 8]>());

        let ptr = Storage::get_mut(&mut multi, handle);
        let ptr = ptr.cast::<usize>().as_ptr();
        assert_eq!(ptr.read(), 0xdead_beef);
    }
}

#[test]
fn test2() {
    #[repr(align(4096))]
    struct Memory([u8; 1 << 24]);
    zst_static! {
        pub struct Zst
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        with struct ZstHandle
        as SingleStackStorage<Memory> = SingleStackStorage::new()
    }

    fn alloc_error_handler(layout: Layout) -> ! { panic!("{:?}", layout) }

    set_alloc_error_handler(alloc_error_handler);
    let store = core::cell::RefCell::new(crate::SingleStackStorage::<[usize; 7]>::new().offsetable());
    let store = &store;
    let alloc = rc::SlimRc::new_in(
        core::cell::RefCell::new(crate::SingleStackStorage::<[usize; 3]>::new().offsetable()),
        store,
    );
    let alloc = alloc.cast::<dyn affix::SharedOffsetHandle<Handle = _>>();
    rc::Rc::new_in(0xdead_beef_usize, alloc.clone());

    let x = BumpStorage::<_, 4096>::new(Zst, 0);
    assert_eq!(x.remaining_space(), (1 << 24));
    x.shared_allocate(Layout::new::<[usize; 32]>()).unwrap();
    assert_eq!(x.remaining_space(), (1 << 24) - 8 * 32);
    assert_eq!(core::mem::size_of_val(&x), 8);
}

#[test]
fn global() {
    use crate::boxed::Box;

    #[repr(align(4096))]
    struct Memory([u8; 1 << 24]);

    fn alloc_error_handler(layout: Layout) -> ! { panic!("{:?}", layout) }

    zst_static!(
        struct Zst
        with struct ZstHandle
        as SingleStackStorage<Memory> = SingleStackStorage::new()
    );

    set_alloc_error_handler(alloc_error_handler);

    install_global_allocator! {
        let GLOBAL: BumpStorage<Zst, 4096> = {
            BumpStorage::new(Zst, core::mem::size_of::<Memory>())
        };
    }

    let bx = Box::new(0xdead_beef_usize);
    let bx2 = Box::new(0xbeef_dead_usize);
    assert_eq!(*bx, 0xdead_beef);
    assert_eq!(*bx2, 0xbeef_dead);
}

// INVARIANTS
//
// * allocate cannot invalidate allocated handles
// * no raw storage function that could deallocate memory may ...
//      * be called concurrently with any other such function the same handle
//      * be called concurrently with any `*get*` function
