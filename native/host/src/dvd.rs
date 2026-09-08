//! DVD filesystem → host directory.
//!
//! Point `MELEE_DATA` at an extracted GALE01 disc tree (or `orig/GALE01`).
//! Paths used by the game are opened relative to that root. No disc image is
//! shipped with this repository.

use std::ffi::{c_char, CStr};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

#[repr(C)]
pub struct DvdFileInfo {
    pub start_addr: u32,
    pub length: u32,
    pub callback: usize,
    pub cb_arg: usize,
    pub handle: u32,
}

struct OpenFile {
    handle: u32,
    file: File,
    length: u32,
}

static FILES: parking_lot::Mutex<Vec<OpenFile>> = parking_lot::const_mutex(Vec::new());

fn data_root() -> PathBuf {
    if let Ok(p) = std::env::var("MELEE_DATA") {
        return PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for cand in [
        cwd.join("data"),
        cwd.join("orig/GALE01"),
        cwd.join("../orig/GALE01"),
        cwd.join("../../orig/GALE01"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    cwd.join("data")
}

fn resolve(path: &str) -> PathBuf {
    let cleaned = path.trim_start_matches('/').replace('\\', "/");
    data_root().join(cleaned)
}

pub fn init() {
    let root = data_root();
    log::info!("DVDInit host root = {}", root.display());
}

pub fn convert_path(path: *const c_char) -> i32 {
    if path.is_null() {
        return -1;
    }
    let s = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    if resolve(&s).is_file() {
        1
    } else {
        -1
    }
}

pub fn open(path: *const c_char, info: *mut DvdFileInfo) -> i32 {
    if path.is_null() || info.is_null() {
        return 0;
    }
    let s = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let p = resolve(&s);
    match File::open(&p) {
        Ok(file) => {
            let length = file.metadata().map(|m| m.len() as u32).unwrap_or(0);
            let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
            FILES.lock().push(OpenFile {
                handle,
                file,
                length,
            });
            unsafe {
                (*info).start_addr = 0;
                (*info).length = length;
                (*info).callback = 0;
                (*info).cb_arg = 0;
                (*info).handle = handle;
            }
            log::debug!("DVDOpen {} ({} bytes)", p.display(), length);
            1
        }
        Err(err) => {
            log::warn!("DVDOpen {} failed: {err}", p.display());
            0
        }
    }
}

pub fn read(info: *mut DvdFileInfo, addr: *mut u8, length: i32, offset: i32) -> i32 {
    if info.is_null() || addr.is_null() || length < 0 {
        return -1;
    }
    let handle = unsafe { (*info).handle };
    let mut files = FILES.lock();
    let Some(slot) = files.iter_mut().find(|f| f.handle == handle) else {
        return -1;
    };
    if slot.file.seek(SeekFrom::Start(offset.max(0) as u64)).is_err() {
        return -1;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(addr, length as usize) };
    match slot.file.read(buf) {
        Ok(n) => n as i32,
        Err(_) => -1,
    }
}

pub fn close(info: *mut DvdFileInfo) -> i32 {
    if info.is_null() {
        return 0;
    }
    let handle = unsafe { (*info).handle };
    FILES.lock().retain(|f| f.handle != handle);
    1
}

pub fn check_path(path: &Path) -> bool {
    path.is_file()
}
