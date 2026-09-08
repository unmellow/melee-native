//! Dolphin OS / VI stand-ins: time, report, cache no-ops, heaps via malloc.

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{c_char, CStr};
use std::io::{self, Write};
use std::ptr;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::Instant;

static START: parking_lot::Mutex<Option<Instant>> = parking_lot::const_mutex(None);
static INTERRUPT: AtomicI32 = AtomicI32::new(1);
static TICK: AtomicU64 = AtomicU64::new(0);

/// GameCube timebase is 40.5 MHz (162 MHz / 4) on NTSC.
pub const OS_BUS_CLOCK: u64 = 162_000_000;
pub const OS_TIMER_CLOCK: u64 = OS_BUS_CLOCK / 4;

pub fn init() {
    let mut g = START.lock();
    if g.is_none() {
        *g = Some(Instant::now());
    }
    log::info!("OSInit (native host)");
}

pub fn now_instant() -> Instant {
    let g = START.lock();
    g.unwrap_or_else(Instant::now)
}

pub fn get_time() -> i64 {
    let elapsed = now_instant().elapsed();
    (elapsed.as_secs_f64() * OS_TIMER_CLOCK as f64) as i64
}

pub fn ticks_to_milliseconds(ticks: i64) -> u32 {
    ((ticks as u64) * 1000 / OS_TIMER_CLOCK) as u32
}

pub fn ticks_to_microseconds(ticks: i64) -> u64 {
    (ticks as u64) * 1_000_000 / OS_TIMER_CLOCK
}

pub fn milliseconds_to_ticks(ms: u32) -> i64 {
    (ms as u64 * OS_TIMER_CLOCK / 1000) as i64
}

pub fn report(fmt: *const c_char) {
    if fmt.is_null() {
        return;
    }
    let s = unsafe { CStr::from_ptr(fmt) }.to_string_lossy();
    let _ = writeln!(io::stderr(), "[OSReport] {s}");
}

pub fn panic(file: *const c_char, line: i32, msg: *const c_char) -> ! {
    let file = if file.is_null() {
        "?".into()
    } else {
        unsafe { CStr::from_ptr(file) }.to_string_lossy().into_owned()
    };
    let msg = if msg.is_null() {
        "panic".into()
    } else {
        unsafe { CStr::from_ptr(msg) }.to_string_lossy().into_owned()
    };
    eprintln!("OSPanic {file}:{line}: {msg}");
    std::process::abort();
}

pub fn disable_interrupts() -> i32 {
    INTERRUPT.swap(0, Ordering::SeqCst)
}

pub fn restore_interrupts(level: i32) {
    INTERRUPT.store(level, Ordering::SeqCst);
}

pub fn enable_interrupts() -> i32 {
    INTERRUPT.swap(1, Ordering::SeqCst)
}

pub fn dc_flush_range(_ptr: *mut u8, _len: u32) {}
pub fn dc_invalidate_range(_ptr: *mut u8, _len: u32) {}
pub fn ic_invalidate_range(_ptr: *mut u8, _len: u32) {}

pub unsafe fn alloc_from_heap(size: u32, align: u32) -> *mut u8 {
    let align = align.max(16) as usize;
    let size = size.max(1) as usize;
    let layout = match Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return ptr::null_mut(),
    };
    alloc(layout)
}

pub unsafe fn free_to_heap(ptr: *mut u8, size: u32, align: u32) {
    if ptr.is_null() {
        return;
    }
    let layout = Layout::from_size_align(size.max(1) as usize, align.max(16) as usize)
        .unwrap_or_else(|_| Layout::from_size_align(1, 16).unwrap());
    dealloc(ptr, layout);
}

pub fn vi_wait_for_retrace() {
    TICK.fetch_add(1, Ordering::Relaxed);
    // Host present path vsyncs; this is a cooperative yield for the game loop.
    std::thread::sleep(std::time::Duration::from_micros(500));
}

pub fn retrace_count() -> u64 {
    TICK.load(Ordering::Relaxed)
}
