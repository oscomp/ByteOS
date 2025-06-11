#![no_std]

pub use spin::{
    lazy::Lazy, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockUpgradableGuard, RwLockWriteGuard,
};
