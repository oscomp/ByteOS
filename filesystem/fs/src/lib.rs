#![no_std]
#![feature(let_chains)]

extern crate alloc;
#[macro_use]
extern crate log;
extern crate bitflags;

pub mod dentry;
pub mod file;
pub mod pathbuf;
pub mod pipe;

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use syscalls::Errno;
use vfscore::VfsResult;
pub use vfscore::{FileType, INodeInterface, SeekFrom};

pub struct WaitBlockingRead<'a>(pub Arc<dyn INodeInterface>, pub &'a mut [u8], pub usize);

impl Future for WaitBlockingRead<'_> {
    type Output = VfsResult<usize>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &mut self.1;
        match file.readat(offset, buffer) {
            Ok(rsize) => Poll::Ready(Ok(rsize)),
            Err(err) => {
                if let Errno::EWOULDBLOCK = err {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}

pub struct WaitBlockingWrite<'a>(pub Arc<dyn INodeInterface>, pub &'a [u8], pub usize);

impl Future for WaitBlockingWrite<'_> {
    type Output = VfsResult<usize>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &self.1;

        match file.writeat(offset, buffer) {
            Ok(wsize) => Poll::Ready(Ok(wsize)),
            Err(err) => {
                if let Errno::EWOULDBLOCK = err {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}
