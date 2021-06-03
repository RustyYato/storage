use core::{
    alloc::Layout,
    cell::Cell,
    marker::{PhantomData, Unsize},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::{self, Pointee, Thin},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    affix::{OffsetHandle, TypedLayoutProvider},
    AffixStorage, Storage,
};

type RcStore<S, I, A> = crate::AffixStorage<TypedLayoutProvider<Counters<I, A>>, TypedLayoutProvider<()>, S>;
type RcHandle<S, I, A> =
    crate::AffixHandle<TypedLayoutProvider<Counters<I, A>>, TypedLayoutProvider<()>, <S as Storage>::Handle>;

pub type SlimRc<T, S = crate::Global> = RefCounted<T, Cell<usize>, One, StrongKind, S>;
pub type Rc<T, S = crate::Global> = RefCounted<T, Cell<usize>, Cell<usize>, StrongKind, S>;
pub type Weak<T, S = crate::Global> = RefCounted<T, Cell<usize>, Cell<usize>, WeakKind, S>;
pub type SlimArc<T, S = crate::Global> = RefCounted<T, AtomicUsize, One, StrongKind, S>;
pub type Arc<T, S = crate::Global> = RefCounted<T, AtomicUsize, AtomicUsize, StrongKind, S>;
pub type Aweak<T, S = crate::Global> = RefCounted<T, AtomicUsize, AtomicUsize, WeakKind, S>;

pub trait Counter {
    const INIT: Self;

    /// # Safety
    ///
    /// `dec` may be called at most `1 + number of increments`
    /// where a single increment is a call to `DynamicCounter::inc`
    /// or `DynamicCounter::inc_if_nonzero` which returns `Some(_)`
    unsafe fn dec(&self, ordering: Ordering) -> usize;

    fn value(&self) -> usize;
}

pub trait DynamicCounter: Counter {
    fn inc(&self, order: Ordering) -> Option<usize>;

    fn inc_if_nonzero(&self, order: Ordering) -> Option<usize>;
}

pub struct One;
impl Counter for One {
    const INIT: Self = Self;

    #[inline]
    unsafe fn dec(&self, _: Ordering) -> usize { 1 }

    #[inline]
    fn value(&self) -> usize { 1 }
}

impl Counter for Cell<usize> {
    const INIT: Self = Self::new(1);

    #[inline]
    unsafe fn dec(&self, _: Ordering) -> usize {
        let count = self.get();
        self.set(count.wrapping_sub(1));
        count
    }

    #[inline]
    fn value(&self) -> usize { self.get() }
}

impl DynamicCounter for Cell<usize> {
    #[inline]
    fn inc(&self, _: Ordering) -> Option<usize> {
        let count = self.get();
        self.set(count.checked_add(1)?);
        Some(count)
    }

    #[inline]
    fn inc_if_nonzero(&self, _: Ordering) -> Option<usize> {
        let count = self.get();
        self.set(count.wrapping_sub(1).checked_add(2)?);
        Some(count)
    }
}

impl Counter for AtomicUsize {
    const INIT: Self = Self::new(1);

    #[inline]
    unsafe fn dec(&self, order: Ordering) -> usize { self.fetch_sub(1, order) }

    #[inline]
    fn value(&self) -> usize { self.load(Ordering::SeqCst) }
}

impl DynamicCounter for AtomicUsize {
    #[inline]
    fn inc(&self, order: Ordering) -> Option<usize> {
        self.fetch_update(order, Ordering::Relaxed, |count| count.checked_add(1))
            .ok()
    }

    #[inline]
    fn inc_if_nonzero(&self, order: Ordering) -> Option<usize> {
        self.fetch_update(order, Ordering::Relaxed, |count| count.wrapping_sub(1).checked_add(2))
            .ok()
    }
}

pub unsafe trait Kind<I, A> {
    const IS_STRONG: bool;
    type Output: Counter;
    type Init: DynamicCounter;
    type Alloc: Counter;

    fn pick<'a>(init: &'a I, alloc: &'a A) -> &'a Self::Output;

    fn init(alloc: &I) -> &Self::Init;

    fn alloc(alloc: &A) -> &Self::Alloc;
}

pub trait DynamicKind<I, A>: Kind<I, A, Output = Self::DynamicOutput> {
    type DynamicOutput: DynamicCounter;
}

pub enum StrongKind {}
pub enum WeakKind {}

unsafe impl<I: DynamicCounter, A: Counter> Kind<I, A> for StrongKind {
    const IS_STRONG: bool = true;
    type Output = I;
    type Init = I;
    type Alloc = A;

    fn pick<'a>(init: &'a I, _: &'a A) -> &'a Self::Output { init }

    fn init(init: &I) -> &Self::Init { init }

    fn alloc(alloc: &A) -> &Self::Alloc { alloc }
}

impl<I: DynamicCounter, A: Counter> DynamicKind<I, A> for StrongKind {
    type DynamicOutput = I;
}

unsafe impl<I: DynamicCounter, A: Counter> Kind<I, A> for WeakKind {
    const IS_STRONG: bool = false;
    type Output = A;
    type Init = I;
    type Alloc = A;

    fn pick<'a>(_: &'a I, alloc: &'a A) -> &'a Self::Output { alloc }

    fn init(init: &I) -> &Self::Init { init }

    fn alloc(alloc: &A) -> &Self::Alloc { alloc }
}

impl<I: DynamicCounter, A: DynamicCounter> DynamicKind<I, A> for WeakKind {
    type DynamicOutput = A;
}

pub struct Counters<I, A> {
    init: I,
    alloc: A,
}

#[repr(C)]
pub struct RecCountInner<T: ?Sized, I, A> {
    counters: Counters<I, A>,
    value: T,
}

pub struct RefCounted<T, I, A, K, S = crate::Global>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: Kind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle,
{
    handle: RcHandle<S, I, A>,
    storage: RcStore<S, I, A>,
    meta: T::Metadata,
    #[allow(clippy::type_complexity)]
    __: PhantomData<(I, A, fn() -> K, T)>,
}

#[inline]
unsafe fn drop_fast<T, I, A, K, S>(storage: &mut RcStore<S, I, A>, handle: RcHandle<S, I, A>, meta: T::Metadata)
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: Kind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle,
{
    let store_ptr = Storage::get(storage, handle);
    let ptr = ptr::from_raw_parts::<T>(store_ptr.as_ptr().cast(), meta);
    let layout = Layout::for_value_raw(ptr);
    let (counters, _) = storage.split(store_ptr, layout);
    let counters = counters.as_ref();
    let counter = K::pick(&counters.init, &counters.alloc);

    if 1 == counter.dec(Ordering::Release) {
        drop_slow::<T, I, A, K, S>(storage, handle, meta, layout)
    }
}

#[cold]
#[allow(clippy::shadow_unrelated)]
fn drop_slow<T, I, A, K, S>(
    storage: &mut RcStore<S, I, A>,
    handle: RcHandle<S, I, A>,
    meta: T::Metadata,
    layout: Layout,
) where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: Kind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle,
{
    if K::IS_STRONG {
        unsafe {
            let mut scope = crate::scope_guard::ScopeGuard::with_extra(storage, move |storage| {
                drop_fast::<T, I, A, WeakKind, S>(storage, handle, meta)
            });

            let storage = scope.extra_mut();

            let ptr = Storage::get_mut(storage, handle);
            let ptr = ptr::from_raw_parts_mut::<T>(ptr.as_ptr().cast(), meta);
            ptr.drop_in_place();
        }
    } else {
        unsafe { storage.deallocate(handle, layout) }
    }
}

impl<T, I, A, K, S> Drop for RefCounted<T, I, A, K, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: Kind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle,
{
    fn drop(&mut self) { unsafe { drop_fast::<T, I, A, K, S>(&mut self.storage, self.handle, self.meta) } }
}

impl<T, I, A, K, S> Clone for RefCounted<T, I, A, K, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: DynamicKind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle + Clone,
{
    fn clone(&self) -> Self {
        let counters = self.counters();
        let counter = K::pick(&counters.init, &counters.alloc);

        counter
            .inc(Ordering::Relaxed)
            .expect("Could not clone a new ref counted pointer");

        let scope = crate::scope_guard::ScopeGuard::new(|| unsafe {
            counter.dec(Ordering::Relaxed);
        });
        let storage = self.storage.clone();
        scope.defuse();

        Self {
            handle: self.handle,
            storage,
            meta: self.meta,
            __: PhantomData,
        }
    }

    fn clone_from(&mut self, other: &Self) {
        let counters = self.counters();
        let counter = K::pick(&counters.init, &counters.alloc);

        counter
            .inc(Ordering::Relaxed)
            .expect("Could not clone a new ref counted pointer");

        let counter = counter as *const K::DynamicOutput;
        let scope = crate::scope_guard::ScopeGuard::new(|| unsafe {
            (*counter).dec(Ordering::Relaxed);
        });
        self.storage.clone_from(&other.storage);
        scope.defuse();
    }
}

impl<T, I, A, K, S> RefCounted<T, I, A, K, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    K: Kind<I, A, Init = I, Alloc = A>,
    S: Storage + OffsetHandle,
{
    fn counters(&self) -> &Counters<I, A> {
        unsafe {
            let store_ptr = self.storage.get(self.handle);
            let ptr = ptr::from_raw_parts::<T>(store_ptr.as_ptr().cast(), self.meta);
            let layout = Layout::for_value_raw(ptr);
            let (counters, _) = self.storage.split(store_ptr, layout);
            &*counters.as_ptr()
        }
    }
}

impl<T, I, A, S> From<crate::boxed::Box<T, RcStore<S, I, A>>> for RefCounted<T, I, A, StrongKind, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    S: Storage + OffsetHandle,
{
    fn from(bx: crate::boxed::Box<T, RcStore<S, I, A>>) -> Self {
        let layout = Layout::for_value::<T>(&bx);
        let (handle, meta, mut storage) = crate::boxed::Box::into_raw_parts(bx);
        unsafe {
            let store_ptr = storage.get_mut(handle);
            let (ptr, _) = storage.split(store_ptr, layout);
            ptr.as_ptr().write(Counters {
                init: Counter::INIT,
                alloc: Counter::INIT,
            });
        }
        Self {
            handle,
            storage,
            meta,
            __: PhantomData,
        }
    }
}

impl<I, A, T> RefCounted<T, I, A, StrongKind>
where
    I: DynamicCounter,
    A: Counter,
    T: Thin,
{
    pub fn new(value: T) -> Self { Self::new_in(value, crate::Global) }
}

impl<I, A, T, S> RefCounted<T, I, A, StrongKind, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Thin,
    S: Storage + OffsetHandle,
{
    pub fn new_in(value: T, storage: S) -> Self { crate::boxed::Box::new_in(value, AffixStorage::new(storage)).into() }
}

impl<I, A, K, T, S> RefCounted<T, I, A, K, S>
where
    I: DynamicCounter,
    A: Counter,
    K: Kind<I, A, Init = I, Alloc = A>,
    T: Pointee + ?Sized,
    S: Storage + OffsetHandle,
{
    pub fn cast<U: ?Sized>(self) -> RefCounted<U, I, A, K, S>
    where
        T: Unsize<U>,
    {
        unsafe {
            let ptr = self.storage.get(self.handle);
            let ptr = ptr::from_raw_parts::<T>(ptr.as_ptr().cast(), self.meta);
            let ptr: *const U = ptr;

            let meta = ptr::metadata(ptr);
            let (handle, _, storage) = Self::into_raw_parts(self);
            RefCounted::from_raw_parts(handle, meta, storage)
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn into_raw_parts(this: Self) -> (RcHandle<S, I, A>, T::Metadata, RcStore<S, I, A>) {
        let this = ManuallyDrop::new(this);
        unsafe {
            let this = ManuallyDrop::new(this);
            let storage = ptr::read(&this.storage);
            (this.handle, this.meta, storage)
        }
    }

    /// # Safety
    ///
    /// `handle` must refer to a valid allocation from `storage`
    /// with a layout that fits `T` with the associated `meta`
    pub unsafe fn from_raw_parts(handle: RcHandle<S, I, A>, meta: T::Metadata, storage: RcStore<S, I, A>) -> Self {
        Self {
            handle,
            storage,
            meta,
            __: PhantomData,
        }
    }
}

impl<I, A, T, S> Deref for RefCounted<T, I, A, StrongKind, S>
where
    I: DynamicCounter,
    A: Counter,
    T: Pointee + ?Sized,
    S: Storage + OffsetHandle,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let store_ptr = self.storage.get(self.handle);
            let ptr = ptr::from_raw_parts::<T>(store_ptr.as_ptr().cast(), self.meta);
            &*ptr
        }
    }
}
#[test]
fn test() {
    static mut SINGLE_THREADED: core::cell::RefCell<crate::OffsetSingleStackStorage<[usize; 3]>> =
        core::cell::RefCell::new(crate::SingleStackStorage::new().offsetable());

    crate::set_alloc_error_handler(|layout| panic!("allocation failurre: {:?}", layout));
    let storage = crate::AffixStorage::new(unsafe { &SINGLE_THREADED });
    let bx = crate::boxed::Box::try_uninit_in(storage).unwrap();
    let bx = crate::boxed::Box::write(bx, 0);
    let x: Rc<usize, _> = Rc::from(bx);
    assert_eq!(core::mem::size_of_val(&x), core::mem::size_of::<usize>());
    let y = x.clone();
    assert!(crate::boxed::Box::<u8, _>::try_uninit_in(storage).is_err());
    assert_eq!(*y, 0);
    drop(x);
    assert!(crate::boxed::Box::<u8, _>::try_uninit_in(storage).is_err());
    drop(y);
    crate::boxed::Box::<u8, _>::try_uninit_in(storage).unwrap();
}
