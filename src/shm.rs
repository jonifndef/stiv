use rustix::fs::{ftruncate, Mode};
use rustix::mm::{mmap, MapFlags, ProtFlags};
use rustix::shm;
use std::ptr::null_mut;
use rand::RngExt;
use std::slice;

pub struct ShmFile {
    shm_path: String,
    ptr: *mut u8,
}

impl ShmFile {
    pub fn new(num_bytes: usize) -> anyhow::Result<ShmFile> {
        let mut rng = rand::rng();
        let randnum: u32 = rng.random();

        let shm_path = format!("/stiv-img-{}", randnum);

        let fd = shm::open(
            &shm_path,
            shm::OFlags::CREATE | shm::OFlags::EXCL | shm::OFlags::RDWR,
            Mode::RUSR | Mode::WUSR,
        )?;

        ftruncate(&fd, num_bytes as u64)?;

        let ptr = unsafe {
            mmap(
                null_mut(),
                num_bytes,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED,
                &fd,
                0,
            )?
        };

        assert!(!ptr.is_null(), "Shm mmap failed");

        Ok(Self {
            shm_path: shm_path,
            ptr: ptr as *mut u8
        })
    }

    pub fn write_to_shm_file(&mut self, data: &[u8]) {
        let buf = unsafe {
            slice::from_raw_parts_mut(self.ptr, data.len())
        };

        buf[..data.len()].copy_from_slice(data);
    }

    pub fn get_shm_path(&self) -> &str {
        self.shm_path.as_str()
    }
}

impl Drop for ShmFile {
    fn drop(&mut self) {
        let _ = shm::unlink(&self.shm_path);
    }
}
