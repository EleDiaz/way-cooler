//! Wrapper around a wl_shm.

use std::{cell::RefCell, fs::File, io::Write as _, os::unix::io::RawFd};

use wayland_client::{
    self,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{self, Format, WlShm}
    },
    NewProxy
};

use crate::area::Size;

/// The minimum version of the wl_shm global to bind to.
pub const WL_SHM_VERSION: u32 = 1;

thread_local! {
    static WL_SHM: RefCell<Option<WlShm>> = RefCell::new(None);
}

impl wayland_client::GlobalImplementor<WlShm> for WlShmManager {
    fn new_global(&mut self, new_proxy: NewProxy<WlShm>) -> WlShm {
        let res = new_proxy.implement_dummy();

        WL_SHM.with(|wl_shm| {
            *wl_shm.borrow_mut() = Some(res.clone());
        });

        res
    }
}

pub struct Buffer {
    pub buffer: WlBuffer,
    pub shared_memory: File
}

impl Buffer {
    pub fn write(&mut self, data: &[u8]) {
        self.shared_memory
            .write(&*data)
            .expect("Could not write data to buffer");
        self.shared_memory.flush().expect("Could not flush buffer");
    }
}

impl AsRef<WlBuffer> for Buffer {
    fn as_ref(&self) -> &WlBuffer {
        &self.buffer
    }
}

pub struct WlShmManager {
    shm: Option<WlShm>
}

impl WlShmManager {
    pub fn new() -> Self {
        WlShmManager { shm: None }
    }

    /// Create a buffer with the given size. May allocate a temporary file.
    pub fn create_buffer(&self, size: Size) -> Result<Buffer, ()> {
        let Size { width, height } = size;
        let width = width as i32;
        let height = height as i32;
        let shm = self
            .shm
            .as_ref()
            .expect("WlShm was not initialized. Make sure your compositor supports the WlShm protocol");

        let temp_file = tempfile::tempfile().expect("Could not make new temp file");
        temp_file
            .set_len(size.width as u64 * size.height as u64 * 4)
            .expect("Could not set file length");
        let fd = std::os::unix::io::AsRawFd::as_raw_fd(&temp_file);

        let pool = shm.create_pool(fd, width * height * 4, NewProxy::implement_dummy)?;
        // TODO ARb32 instead
        pool.create_buffer(
            0,
            width,
            height,
            width * 4,
            Format::Argb8888,
            NewProxy::implement_dummy
        )
        .map(|buffer| Buffer {
            shared_memory: temp_file,
            buffer
        })
    }
}

/// Create a buffer from the raw file descriptor in the given size.
///
/// This should be called from a shell and generally should not be used
/// directly by the Awesome objects.
pub fn create_buffer(fd: RawFd, size: Size) -> Result<WlBuffer, ()> {
    let Size { width, height } = size;
    let width = width as i32;
    let height = height as i32;
    WL_SHM.with(|wl_shm| {
        let wl_shm = wl_shm.borrow();
        let wl_shm = wl_shm.as_ref().expect("WL_SHM was not initilized");
        let pool = wl_shm.create_pool(fd, width * height * 4, NewProxy::implement_dummy)?;
        // TODO ARb32 instead
        pool.create_buffer(
            0,
            width,
            height,
            width * 4,
            wl_shm::Format::Argb8888,
            NewProxy::implement_dummy
        )
    })
}
