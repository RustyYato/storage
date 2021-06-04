//! Taken from `crossbeam_utils`
//!
//! [`crossbeam_utils::Backoff`](https://docs.rs/crossbeam-utils/0.8.5/crossbeam_utils/struct.Backoff.html)

use core::{cell::Cell, fmt};

const SPIN_LIMIT: u32 = 6;
const YIELD_LIMIT: u32 = 10;

pub struct Backoff {
    step: Cell<u32>,
}

impl Backoff {
    #[inline]
    pub const fn new() -> Self { Self { step: Cell::new(0) } }

    #[inline]
    #[allow(unused)]
    pub fn reset(&self) { self.step.set(0); }

    #[inline]
    pub fn spin(&self) -> bool {
        for _ in 0..1 << self.step.get().min(SPIN_LIMIT) {
            core::hint::spin_loop();
        }

        if self.step.get() <= SPIN_LIMIT {
            self.step.set(self.step.get() + 1);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn is_completed(&self) -> bool { self.step.get() > YIELD_LIMIT }
}

impl fmt::Debug for Backoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backoff")
            .field("step", &self.step)
            .field("is_completed", &self.is_completed())
            .finish()
    }
}

impl Default for Backoff {
    fn default() -> Self { Self::new() }
}
