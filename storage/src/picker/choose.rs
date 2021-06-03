use core::{
    alloc::Layout,
    ops::{BitAnd, BitOr, Not},
};

pub unsafe trait Choose: Copy {
    fn choose(&self, layout: Layout) -> bool;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MaxSize<const VALUE: usize>;
#[derive(Default, Debug, Clone, Copy)]
pub struct MinSize<const VALUE: usize>;
#[derive(Default, Debug, Clone, Copy)]
pub struct MaxAlign<const VALUE: usize>;
#[derive(Default, Debug, Clone, Copy)]
pub struct MinAlign<const VALUE: usize>;
#[derive(Default, Debug, Clone, Copy)]
pub struct NotC<T>(pub T);
#[derive(Default, Debug, Clone, Copy)]
pub struct AndC<A, B>(pub A, pub B);
#[derive(Default, Debug, Clone, Copy)]
pub struct OrC<A, B>(pub A, pub B);

macro_rules! impl_op {
    (AND ($($generics:tt)*) $type:ty) => {
        impl<F: Choose, $($generics)*> BitAnd<F> for $type {
            type Output = AndC<Self, F>;

            #[inline]
            fn bitand(self, other: F) -> Self::Output {
                AndC(self, other)
            }
        }
    };
    (OR ($($generics:tt)*) $type:ty) => {
        impl<F: Choose, $($generics)*> BitOr<F> for $type {
            type Output = OrC<Self, F>;

            #[inline]
            fn bitor(self, other: F) -> Self::Output {
                OrC(self, other)
            }
        }
    };
    (NOT ($($generics:tt)*) $type:ty) => {
        impl<$($generics)*> Not for $type {
            type Output = NotC<Self>;

            #[inline]
            fn not(self) -> Self::Output {
                NotC(self)
            }
        }
    };
}

macro_rules! impl_ops {
    (($($generics:tt)*) $type:ty) => {
        impl_ops!(($($generics)*) $type, (AND OR NOT));
    };
    (($($generics:tt)*) $type:ty, ()) => {};
    (($($generics:tt)*) $type:ty, ($op:ident $($ops:ident)*)) => {
        impl_op!($op ($($generics)*) $type);
        impl_ops!(($($generics)*) $type, ($($ops)*));
    };
}

impl_ops!((const VALUE: usize) MaxSize<VALUE>);
impl_ops!((const VALUE: usize) MinSize<VALUE>);
impl_ops!((const VALUE: usize) MaxAlign<VALUE>);
impl_ops!((const VALUE: usize) MinAlign<VALUE>);
impl_ops!((A, B) AndC<A, B>, (AND OR));
impl_ops!((A, B) OrC<A, B>, (AND OR));
impl_ops!((A) NotC<A>, (AND OR));

impl<F: Choose> Not for NotC<F> {
    type Output = F;

    fn not(self) -> Self::Output { self.0 }
}

impl<A: Choose + Not, B: Choose + Not> Not for AndC<A, B> {
    type Output = OrC<A::Output, B::Output>;

    fn not(self) -> Self::Output {
        let Self(a, b) = self;
        OrC(!a, !b)
    }
}

impl<A: Choose + Not, B: Choose + Not> Not for OrC<A, B> {
    type Output = AndC<A::Output, B::Output>;

    fn not(self) -> Self::Output {
        let Self(a, b) = self;
        AndC(!a, !b)
    }
}

unsafe impl<const VALUE: usize> Choose for MaxSize<VALUE> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool { layout.size() <= VALUE }
}

unsafe impl<const VALUE: usize> Choose for MinSize<VALUE> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool { layout.size() >= VALUE }
}

unsafe impl<const VALUE: usize> Choose for MaxAlign<VALUE> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool { layout.align() <= VALUE }
}

unsafe impl<const VALUE: usize> Choose for MinAlign<VALUE> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool { layout.align() >= VALUE }
}

unsafe impl<A: Choose, B: Choose> Choose for AndC<A, B> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool {
        let Self(a, b) = self;
        a.choose(layout) && b.choose(layout)
    }
}

unsafe impl<A: Choose, B: Choose> Choose for OrC<A, B> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool {
        let Self(a, b) = self;
        a.choose(layout) || b.choose(layout)
    }
}

unsafe impl<A: Choose> Choose for NotC<A> {
    #[inline]
    fn choose(&self, layout: Layout) -> bool {
        let Self(a) = self;
        !a.choose(layout)
    }
}
