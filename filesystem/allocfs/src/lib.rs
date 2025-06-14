#![no_std]
#![feature(extract_if)]
extern crate alloc;

use alloc::{string::String, sync::Arc, vec::Vec};
use core::cmp::min;
use libc_core::{
    consts::UTIME_OMIT,
    types::{Stat, StatMode, TimeSpec},
};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, VfsResult};

pub struct AllocFS {
    root: Arc<FSDirInner>,
}

impl AllocFS {
    pub fn new() -> Arc<Self> {
        let inner = Arc::new(FSDirInner {
            name: String::from(""),
            children: Mutex::new(Vec::new()),
        });
        Arc::new(Self { root: inner })
    }
}

impl FileSystem for AllocFS {
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        Arc::new(FSDir {
            inner: self.root.clone(),
        })
    }

    fn name(&self) -> &str {
        "allocfs"
    }
}

pub struct FSDirInner {
    name: String,
    children: Mutex<Vec<FileContainer>>,
}

// TODO: use frame insteads of Vec.
pub struct FSFileInner {
    name: String,
    content: Mutex<Vec<u8>>,
    times: Mutex<[TimeSpec; 3]>, // ctime, atime, mtime.
}

#[allow(dead_code)]
pub struct FSLinkInner {
    name: String,
    link_file: Arc<dyn INodeInterface>,
}

pub enum FileContainer {
    File(Arc<FSFileInner>),
    Dir(Arc<FSDirInner>),
    Link(Arc<FSLinkInner>),
}

impl FileContainer {
    #[inline]
    fn to_inode(&self) -> VfsResult<Arc<dyn INodeInterface>> {
        match self {
            FileContainer::File(file) => Ok(Arc::new(FSFile {
                inner: file.clone(),
            })),
            FileContainer::Dir(dir) => Ok(Arc::new(FSDir { inner: dir.clone() })),
            FileContainer::Link(link) => Ok(Arc::new(FSLink {
                inner: link.clone(),
                link_file: link.link_file.clone(),
            })),
        }
    }

    #[inline]
    fn filename(&self) -> &str {
        match self {
            FileContainer::File(file) => &file.name,
            FileContainer::Dir(dir) => &dir.name,
            FileContainer::Link(link) => &link.name,
        }
    }
}

#[allow(dead_code)]
pub struct FSLink {
    inner: Arc<FSLinkInner>,
    link_file: Arc<dyn INodeInterface>,
}

pub struct FSDir {
    inner: Arc<FSDirInner>,
}

impl FSDir {
    pub const fn new(inner: Arc<FSDirInner>) -> Self {
        Self { inner }
    }
}

impl INodeInterface for FSDir {
    fn mkdir(&self, name: &str) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(Errno::EEXIST))?;

        let new_inner = Arc::new(FSDirInner {
            name: String::from(name),
            children: Mutex::new(Vec::new()),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Dir(new_inner));

        Ok(())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode())
            .ok_or(Errno::ENOENT)?
    }

    fn create(&self, name: &str, ty: FileType) -> VfsResult<()> {
        if ty == FileType::Directory {
            let new_inner = Arc::new(FSDirInner {
                name: String::from(name),
                children: Mutex::new(Vec::new()),
            });
            self.inner
                .children
                .lock()
                .push(FileContainer::Dir(new_inner.clone()));
            Ok(())
        } else if ty == FileType::File {
            let new_inner = Arc::new(FSFileInner {
                name: String::from(name),
                content: Mutex::new(Vec::new()),
                times: Mutex::new([Default::default(); 3]),
            });
            self.inner
                .children
                .lock()
                .push(FileContainer::File(new_inner.clone()));
            Ok(())
        } else {
            unimplemented!("")
        }
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        // TODO: identify whether the dir is empty(through metadata.childrens)
        // return DirectoryNotEmpty if not empty.
        let len = self
            .inner
            .children
            .lock()
            .extract_if(.., |x| match x {
                FileContainer::Dir(x) => x.name == name,
                _ => false,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(Errno::ENOENT),
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .children
            .lock()
            .iter()
            .map(|x| match x {
                FileContainer::File(file) => DirEntry {
                    filename: file.name.clone(),
                    len: file.content.lock().len(),
                    file_type: FileType::File,
                },
                FileContainer::Dir(dir) => DirEntry {
                    filename: dir.name.clone(),
                    len: 0,
                    file_type: FileType::Directory,
                },
                FileContainer::Link(link) => DirEntry {
                    filename: link.name.clone(),
                    len: 0,
                    file_type: FileType::Link,
                },
            })
            .collect())
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        let len = self
            .inner
            .children
            .lock()
            .extract_if(.., |x| match x {
                FileContainer::File(x) => x.name == name,
                FileContainer::Dir(_) => false,
                FileContainer::Link(x) => x.name == name,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(Errno::ENOENT),
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        self.remove(name)
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.mtime = Default::default();
        stat.atime = Default::default();
        stat.ctime = Default::default();
        Ok(())
    }

    fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(Errno::EEXIST))?;

        let new_inner = Arc::new(FSLinkInner {
            name: String::from(name),
            link_file: src,
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Link(new_inner));

        Ok(())
    }
}

pub struct FSFile {
    inner: Arc<FSFileInner>,
}

impl FSFile {
    pub const fn new(inner: Arc<FSFileInner>) -> Self {
        Self { inner }
    }
}

impl INodeInterface for FSFile {
    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let file_size = self.inner.content.lock().len();
        match offset >= file_size {
            true => Ok(0),
            false => {
                let read_len = min(buffer.len(), file_size - offset);
                let content = self.inner.content.lock();
                buffer[..read_len].copy_from_slice(&content[offset..(offset + read_len)]);
                Ok(read_len)
            }
        }
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        if self.inner.content.lock().len() < offset + buffer.len() {
            self.inner.content.lock().resize(offset + buffer.len(), 0);
        }
        self.inner.content.lock()[offset..].copy_from_slice(buffer);
        Ok(buffer.len())
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.inner.content.lock().drain(size..);
        Ok(())
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        log::debug!("stat ramfs");
        // stat.ino = 1; // TODO: convert path to number(ino)
        if self.inner.name.ends_with(".s") {
            stat.ino = 2; // TODO: convert path to number(ino)
        } else {
            stat.ino = 1; // TODO: convert path to number(ino)
        }
        stat.mode = StatMode::FILE; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = self.inner.content.lock().len() as u64;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id

        stat.atime = self.inner.times.lock()[1];
        stat.mtime = self.inner.times.lock()[2];
        Ok(())
    }

    fn utimes(&self, times: &mut [TimeSpec]) -> VfsResult<()> {
        if times[0].nsec != UTIME_OMIT {
            self.inner.times.lock()[1] = times[0];
        }
        if times[1].nsec != UTIME_OMIT {
            self.inner.times.lock()[2] = times[1];
        }
        Ok(())
    }
}

impl INodeInterface for FSLink {
    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        // self.link_file.stat(stat)
        stat.ino = self as *const FSLink as u64;
        stat.blksize = 4096;
        stat.blocks = 8;
        stat.size = 3;
        stat.uid = 0;
        stat.gid = 0;
        stat.mode = StatMode::LINK;
        Ok(())
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        self.link_file.readat(offset, buffer)
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        self.link_file.writeat(offset, buffer)
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.link_file.truncate(size)
    }
}
