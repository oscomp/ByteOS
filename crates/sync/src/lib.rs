#![no_std]

pub use spin::{
    lazy::Lazy, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockUpgradableGuard, RwLockWriteGuard,
};

use core::cell::UnsafeCell;
use core::fmt;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct LazyInit<T> {
    inited: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send + Sync> Sync for LazyInit<T> {}
unsafe impl<T: Send> Send for LazyInit<T> {}

impl<T> LazyInit<T> {
    pub const fn new() -> Self {
        Self {
            inited: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn init_by(&self, data: T) {
        assert!(!self.is_init());
        unsafe { (*self.data.get()).as_mut_ptr().write(data) };
        self.inited.store(true, Ordering::Release);
    }

    pub fn is_init(&self) -> bool {
        self.inited.load(Ordering::Acquire)
    }

    pub fn try_get(&self) -> Option<&T> {
        if self.is_init() {
            unsafe { Some(&*(*self.data.get()).as_ptr()) }
        } else {
            None
        }
    }

    fn check_init(&self) {
        if !self.is_init() {
            panic!(
                "Use uninitialized value: {:?}",
                core::any::type_name::<Self>()
            )
        }
    }

    #[inline]
    fn get(&self) -> &T {
        self.check_init();
        unsafe { self.get_unchecked() }
    }

    #[inline]
    fn get_mut(&mut self) -> &mut T {
        self.check_init();
        unsafe { self.get_mut_unchecked() }
    }

    /// # Safety
    ///
    /// Must be called after initialization.
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        &*(*self.data.get()).as_ptr()
    }

    /// # Safety
    ///
    /// Must be called after initialization.
    #[inline]
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        &mut *(*self.data.get()).as_mut_ptr()
    }
}

impl<T> Default for LazyInit<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for LazyInit<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_get() {
            Some(s) => write!(f, "LazyInit {{ data: ")
                .and_then(|()| s.fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "LazyInit {{ <uninitialized> }}"),
        }
    }
}

impl<T> Deref for LazyInit<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        self.get()
    }
}

impl<T> DerefMut for LazyInit<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<T> Drop for LazyInit<T> {
    fn drop(&mut self) {
        if self.is_init() {
            unsafe { core::ptr::drop_in_place((*self.data.get()).as_mut_ptr()) };
        }
    }
}
