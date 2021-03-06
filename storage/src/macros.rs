mod zst_static_with;

mod install_global;
mod zst_static;

pub use core;
use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{MemoryBlock, NonEmptyMemoryBlock};
pub type MbR<H> = Result<crate::MemoryBlock<H>, crate::AllocErr>;
pub type NeMbR<H> = Result<crate::NonEmptyMemoryBlock<H>, crate::AllocErr>;

pub struct Once(AtomicU8);

pub struct Finisher<'once> {
    inner: &'once Once,
}

impl Once {
    pub const fn new() -> Self { Self(AtomicU8::new(0)) }

    pub fn is_done(&self) -> bool { self.0.load(Ordering::Acquire) == 2 }

    pub fn attempt(&self) -> Option<Finisher<'_>> {
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Finisher { inner: self })
    }
}

impl Finisher<'_> {
    pub fn finish(&self) { self.inner.0.store(2, Ordering::Release); }
}

pub struct MacroToken(());

impl MacroToken {
    pub const unsafe fn new() -> Self { Self(()) }
}

pub fn map_mbr<A, B, F: FnOnce(A) -> B>(a: MbR<A>, f: F) -> MbR<B> {
    a.map(move |memory_block| MemoryBlock {
        handle: f(memory_block.handle),
        size: memory_block.size,
    })
}

pub fn map_nembr<A, B, F: FnOnce(A) -> B>(a: NeMbR<A>, f: F) -> NeMbR<B> {
    a.map(move |memory_block| NonEmptyMemoryBlock {
        handle: f(memory_block.handle),
        size: memory_block.size,
    })
}

pub fn assert_thread_safe<T: Send + Sync>() {}

#[allow(clippy::missing_const_for_fn)]
pub unsafe fn assume_init_ref<T>(m: &MaybeUninit<T>) -> &T { &*m.as_ptr() }

#[cold]
#[inline(never)]
pub fn could_not_init() -> ! { core::panic!("Could not initialize global allocator") }
