//! C ABI matching Dolphin SDK symbols the decomp already calls.

use std::ffi::c_char;

use crate::dvd::{self, DvdFileInfo};
use crate::gx::{self, GxState};
use crate::os;
use crate::pad::{self, PadStatus, PadSystem, PAD_CHAN0_BIT};

static GX: parking_lot::Mutex<Option<GxState>> = parking_lot::const_mutex(None);
static PAD: parking_lot::Mutex<Option<PadSystem>> = parking_lot::const_mutex(None);

fn pad() -> parking_lot::MutexGuard<'static, Option<PadSystem>> {
    let mut g = PAD.lock();
    if g.is_none() {
        *g = Some(PadSystem::new());
    }
    g
}

fn gx() -> parking_lot::MappedMutexGuard<'static, GxState> {
    let mut g = GX.lock();
    if g.is_none() {
        *g = Some(GxState::new());
    }
    parking_lot::MutexGuard::map(g, |o| o.as_mut().unwrap())
}

// --- PAD ---

#[no_mangle]
pub extern "C" fn PADInit() -> i32 {
    let _ = pad();
    1
}

#[no_mangle]
pub extern "C" fn PADRead(status: *mut PadStatus) -> u32 {
    if status.is_null() {
        return 0;
    }
    let mut g = pad();
    let out = g.as_mut().unwrap().poll();
    unsafe {
        std::ptr::copy_nonoverlapping(out.as_ptr(), status, 4);
    }
    0
}

#[no_mangle]
pub extern "C" fn PADClamp(status: *mut PadStatus) {
    if status.is_null() {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(status, 4) };
    pad::clamp_slice(slice);
}

#[no_mangle]
pub extern "C" fn PADControlMotor(chan: i32, command: u32) {
    if let Some(p) = pad().as_mut() {
        p.set_motor(chan, command);
    }
}

#[no_mangle]
pub extern "C" fn PADControlAllMotors(commands: *const u32) {
    if commands.is_null() {
        return;
    }
    let cmds = unsafe { std::slice::from_raw_parts(commands, 4) };
    if let Some(p) = pad().as_mut() {
        for (i, c) in cmds.iter().enumerate() {
            p.set_motor(i as i32, *c);
        }
    }
}

#[no_mangle]
pub extern "C" fn PADReset(_mask: u32) -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn PADRecalibrate(_mask: u32) -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn PADSetSpec(_spec: u32) {}

#[no_mangle]
pub extern "C" fn PADGetSpec() -> u32 {
    5
}

#[no_mangle]
pub extern "C" fn PADSync() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn PADSetSamplingRate(_msec: u32) {}

#[no_mangle]
pub extern "C" fn PADSetAnalogMode(_mode: u32) {}

// --- OS ---

#[no_mangle]
pub extern "C" fn OSInit() {
    os::init();
}

#[no_mangle]
pub extern "C" fn OSGetTime() -> i64 {
    os::get_time()
}

#[no_mangle]
pub extern "C" fn OSGetTick() -> u32 {
    os::get_time() as u32
}

#[no_mangle]
pub extern "C" fn OSTicksToMilliseconds(ticks: i64) -> u32 {
    os::ticks_to_milliseconds(ticks)
}

#[no_mangle]
pub extern "C" fn OSDisableInterrupts() -> i32 {
    os::disable_interrupts()
}

#[no_mangle]
pub extern "C" fn OSRestoreInterrupts(level: i32) {
    os::restore_interrupts(level);
}

#[no_mangle]
pub extern "C" fn OSEnableInterrupts() -> i32 {
    os::enable_interrupts()
}

#[no_mangle]
pub unsafe extern "C" fn OSReport(fmt: *const c_char) {
    os::report(fmt);
}

#[no_mangle]
pub unsafe extern "C" fn OSPanic(file: *const c_char, line: i32, msg: *const c_char) -> ! {
    os::panic(file, line, msg)
}

#[no_mangle]
pub extern "C" fn DCFlushRange(ptr: *mut u8, len: u32) {
    os::dc_flush_range(ptr, len);
}

#[no_mangle]
pub extern "C" fn DCInvalidateRange(ptr: *mut u8, len: u32) {
    os::dc_invalidate_range(ptr, len);
}

#[no_mangle]
pub extern "C" fn ICInvalidateRange(ptr: *mut u8, len: u32) {
    os::ic_invalidate_range(ptr, len);
}

// --- VI ---

#[no_mangle]
pub extern "C" fn VIInit() {}

#[no_mangle]
pub extern "C" fn VIWaitForRetrace() {
    os::vi_wait_for_retrace();
}

#[no_mangle]
pub extern "C" fn VIFlush() {}

#[no_mangle]
pub extern "C" fn VISetNextFrameBuffer(_fb: *mut u8) {}

#[no_mangle]
pub extern "C" fn VIConfigure(_rmode: *const u8) {}

#[no_mangle]
pub extern "C" fn VISetBlack(_black: i32) {}

#[no_mangle]
pub extern "C" fn VIGetRetraceCount() -> u32 {
    os::retrace_count() as u32
}

// --- DVD ---

#[no_mangle]
pub extern "C" fn DVDInit() {
    dvd::init();
}

#[no_mangle]
pub extern "C" fn DVDOpen(path: *const c_char, info: *mut DvdFileInfo) -> i32 {
    dvd::open(path, info)
}

#[no_mangle]
pub extern "C" fn DVDReadPrio(
    info: *mut DvdFileInfo,
    addr: *mut u8,
    length: i32,
    offset: i32,
    _prio: i32,
) -> i32 {
    dvd::read(info, addr, length, offset)
}

#[no_mangle]
pub extern "C" fn DVDClose(info: *mut DvdFileInfo) -> i32 {
    dvd::close(info)
}

#[no_mangle]
pub extern "C" fn DVDConvertPathToEntrynum(path: *const c_char) -> i32 {
    dvd::convert_path(path)
}

#[no_mangle]
pub extern "C" fn DVDCheckDisk() -> i32 {
    1
}

// --- GX ---

#[no_mangle]
pub extern "C" fn GXInit(_base: *mut u8, _size: u32) -> *mut u8 {
    gx().init();
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn GXSetViewport(x: f32, y: f32, w: f32, h: f32, n: f32, f: f32) {
    gx().set_viewport(x, y, w, h, n, f);
}

#[no_mangle]
pub extern "C" fn GXSetScissor(x: u32, y: u32, w: u32, h: u32) {
    gx().set_scissor(x, y, w, h);
}

#[no_mangle]
pub extern "C" fn GXSetProjection(mtx: *const f32, kind: u32) {
    if mtx.is_null() {
        return;
    }
    let m = unsafe { *(mtx as *const [f32; 16]) };
    gx().set_projection(&m, kind as u8);
}

#[no_mangle]
pub extern "C" fn GXLoadPosMtxImm(mtx: *const f32, id: u32) {
    if mtx.is_null() {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(mtx, 12) };
    gx().load_pos_mtx(slice, id);
}

#[no_mangle]
pub extern "C" fn GXLoadNrmMtxImm(mtx: *const f32, id: u32) {
    GXLoadPosMtxImm(mtx, id);
}

#[no_mangle]
pub extern "C" fn GXSetCurrentMtx(_id: u32) {}

#[no_mangle]
pub extern "C" fn GXSetCullMode(mode: u32) {
    gx().set_cull(mode as u8);
}

#[no_mangle]
pub extern "C" fn GXSetBlendMode(kind: u32, src: u32, dst: u32, logic: u32) {
    gx().set_blend(kind as u8, src as u8, dst as u8, logic as u8);
}

#[no_mangle]
pub extern "C" fn GXSetZMode(enable: u8, func: u32, write: u8) {
    gx().set_z_mode(enable != 0, func as u8, write != 0);
}

#[no_mangle]
pub extern "C" fn GXSetAlphaCompare(_comp0: u32, _ref0: u8, _op: u32, _comp1: u32, _ref1: u8) {}

#[no_mangle]
pub extern "C" fn GXSetTevOp(stage: u32, op: u32) {
    gx().set_tev_op(stage as u8, op as u8);
}

#[no_mangle]
pub extern "C" fn GXSetNumTevStages(n: u8) {
    gx().set_num_tev(n);
}

#[no_mangle]
pub extern "C" fn GXSetTevOrder(_stage: u32, _coord: u32, _map: u32, _chan: u32) {}

#[no_mangle]
pub extern "C" fn GXSetNumChans(n: u8) {
    gx().set_chan_ctrl(0, n > 0);
}

#[no_mangle]
pub extern "C" fn GXSetChanCtrl(
    chan: u32,
    enable: u8,
    _amb: u32,
    _mat: u32,
    _light: u32,
    _diff: u32,
    _attn: u32,
) {
    gx().set_chan_ctrl(chan as u8, enable != 0);
}

#[no_mangle]
pub extern "C" fn GXSetCopyClear(color: u32, z: u32) {
    let r = ((color >> 24) & 0xff) as u8;
    let g = ((color >> 16) & 0xff) as u8;
    let b = ((color >> 8) & 0xff) as u8;
    let a = (color & 0xff) as u8;
    let zf = if z == 0xffffff { 1.0 } else { z as f32 / 16_777_215.0 };
    gx().copy_clear(r, g, b, a, zf);
}

#[no_mangle]
pub extern "C" fn GXBegin(prim: u32, fmt: u32, nverts: u16) {
    gx().begin(prim as u8, fmt as u8, nverts);
}

#[no_mangle]
pub extern "C" fn GXPosition3f32(x: f32, y: f32, z: f32) {
    gx().position3f32(x, y, z);
}

#[no_mangle]
pub extern "C" fn GXPosition3s16(x: i16, y: i16, z: i16) {
    gx().position3s16(x, y, z);
}

#[no_mangle]
pub extern "C" fn GXNormal3f32(x: f32, y: f32, z: f32) {
    gx().normal3f32(x, y, z);
}

#[no_mangle]
pub extern "C" fn GXColor4u8(r: u8, g: u8, b: u8, a: u8) {
    gx().color4u8(r, g, b, a);
}

#[no_mangle]
pub extern "C" fn GXColor1u32(rgba: u32) {
    gx().color1u32(rgba);
}

#[no_mangle]
pub extern "C" fn GXTexCoord2f32(u: f32, v: f32) {
    gx().texcoord2f32(u, v);
}

#[no_mangle]
pub extern "C" fn GXEnd() {
    gx().end();
}

#[no_mangle]
pub extern "C" fn GXDrawDone() {}

#[no_mangle]
pub extern "C" fn GXAbortFrame() {
    gx().abort_frame();
}

#[no_mangle]
pub extern "C" fn GXInvalidateTexAll() {}

#[no_mangle]
pub extern "C" fn GXCopyDisp(_dest: *mut u8, _clear: u8) {}

#[no_mangle]
pub extern "C" fn GXSetVtxDesc(_attr: u32, _kind: u32) {}

#[no_mangle]
pub extern "C" fn GXClearVtxDesc() {}

#[no_mangle]
pub extern "C" fn GXSetVtxAttrFmt(_fmt: u32, _attr: u32, _cnt: u32, _kind: u32, _frac: u32) {}

#[no_mangle]
pub extern "C" fn GXSetNumTexGens(n: u32) {
    let _ = n;
}

#[no_mangle]
pub extern "C" fn GXSetTexCoordGen(_dst: u32, _fn: u32, _src: u32, _mtx: u32) {}

#[no_mangle]
pub extern "C" fn GXPixModeSync() {}

#[no_mangle]
pub extern "C" fn GXFlush() {}

#[no_mangle]
pub extern "C" fn GXSetDrawSync(_token: u16) {}

#[no_mangle]
pub extern "C" fn GXSetColorUpdate(enable: u8) {
    let _ = enable;
}

#[no_mangle]
pub extern "C" fn GXSetAlphaUpdate(enable: u8) {
    let _ = enable;
}

#[no_mangle]
pub extern "C" fn GXSetDither(enable: u8) {
    let _ = enable;
}

#[no_mangle]
pub extern "C" fn GXSetFog(_kind: u32, _start: f32, _end: f32, _near: f32, _far: f32, _color: u32) {}

#[no_mangle]
pub extern "C" fn GXSetLineWidth(width: u8, _tex: u32) {
    let _ = width;
}

#[no_mangle]
pub extern "C" fn GXSetCoPlanar(enable: u8) {
    let _ = enable;
}

#[no_mangle]
pub extern "C" fn GXSetClipMode(mode: u32) {
    let _ = mode;
}

// --- MTX (from dolphin/mtx.h) ---

#[no_mangle]
pub extern "C" fn MTXIdentity(m: *mut f32) {
    if m.is_null() {
        return;
    }
    let id = gx::mtx_identity();
    unsafe {
        std::ptr::copy_nonoverlapping(id.as_ptr(), m, 12);
    }
}

#[no_mangle]
pub extern "C" fn MTXConcat(a: *const f32, b: *const f32, o: *mut f32) {
    if a.is_null() || b.is_null() || o.is_null() {
        return;
    }
    let aa = unsafe { *(a as *const [f32; 12]) };
    let bb = unsafe { *(b as *const [f32; 12]) };
    let r = gx::mtx_concat(&aa, &bb);
    unsafe {
        std::ptr::copy_nonoverlapping(r.as_ptr(), o, 12);
    }
}

#[no_mangle]
pub extern "C" fn MTXTrans(m: *mut f32, x: f32, y: f32, z: f32) {
    if m.is_null() {
        return;
    }
    let t = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
    unsafe {
        std::ptr::copy_nonoverlapping(t.as_ptr(), m, 12);
    }
}

#[no_mangle]
pub extern "C" fn C_MTXPerspective(m: *mut f32, fov_y: f32, aspect: f32, n: f32, f: f32) {
    if m.is_null() {
        return;
    }
    let p = gx::mtx_perspective(fov_y, aspect, n, f);
    unsafe {
        std::ptr::copy_nonoverlapping(p.as_ptr(), m, 16);
    }
}

#[no_mangle]
pub extern "C" fn C_MTXOrtho(m: *mut f32, t: f32, b: f32, l: f32, r: f32, n: f32, f: f32) {
    if m.is_null() {
        return;
    }
    let p = glam::Mat4::orthographic_rh(l, r, b, t, n, f).to_cols_array();
    unsafe {
        std::ptr::copy_nonoverlapping(p.as_ptr(), m, 16);
    }
}

#[no_mangle]
pub extern "C" fn PAD_CHAN_BIT(i: u32) -> u32 {
    PAD_CHAN0_BIT >> i
}
