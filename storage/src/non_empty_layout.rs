use core::{
    alloc::{Layout, LayoutError},
    num::NonZeroUsize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyLayout {
    size: NonZeroUsize,
    align: NonZeroUsize,
}

impl NonEmptyLayout {
    #[allow(clippy::manual_map)]
    #[must_use = "calling `new` without using result"]
    pub const fn new(layout: Layout) -> Option<Self> {
        match NonZeroUsize::new(layout.size()) {
            None => None,
            Some(size) => Some(Self {
                size,
                align: unsafe { NonZeroUsize::new_unchecked(layout.align()) },
            }),
        }
    }

    /// # Safety
    ///
    /// `layout.size() != 0`
    #[must_use = "calling `new_unchecked` without using result"]
    pub const unsafe fn new_unchecked(layout: Layout) -> Self {
        Self {
            size: NonZeroUsize::new_unchecked(layout.size()),
            align: NonZeroUsize::new_unchecked(layout.align()),
        }
    }

    pub const fn size(self) -> usize { self.size.get() }

    pub const fn align(self) -> usize { self.align.get() }

    /// # Errors
    ///
    /// On arithmetic overflow, returns `LayoutError`.
    pub fn extend(self, other: Layout) -> Result<(Self, usize), LayoutError> {
        Layout::from(self)
            .extend(other)
            .map(|(layout, offset)| (unsafe { Self::new_unchecked(layout) }, offset))
    }

    /// # Errors
    ///
    /// On arithmetic overflow, returns `LayoutError`.
    pub fn extend_after(self, other: Layout) -> Result<(Self, NonZeroUsize), LayoutError> {
        other
            .extend(Layout::from(self))
            .map(|(layout, offset)| unsafe { (Self::new_unchecked(layout), NonZeroUsize::new_unchecked(offset)) })
    }
}

impl From<NonEmptyLayout> for Layout {
    fn from(layout: NonEmptyLayout) -> Self {
        unsafe { Self::from_size_align_unchecked(layout.size(), layout.align()) }
    }
}
