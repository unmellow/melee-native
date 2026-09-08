//! Native PAD / SI replacement.
//!
//! Produces `PADStatus` matching `extern/dolphin/include/dolphin/pad.h` so the
//! decomp's `HSD_PadRenewRawStatus` can consume host controllers unchanged.
//!
//! Backends, in priority order per port:
//! 1. Nintendo Wii U GameCube adapter (USB hidraw, VID:PID 057e:0337)
//! 2. Linux joystick devices (`/dev/input/js*`) with console-pad maps
//! 3. Keyboard (port 0 fallback)

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const PAD_MAX_CONTROLLERS: usize = 4;

pub const PAD_BUTTON_LEFT: u16 = 1 << 0;
pub const PAD_BUTTON_RIGHT: u16 = 1 << 1;
pub const PAD_BUTTON_DOWN: u16 = 1 << 2;
pub const PAD_BUTTON_UP: u16 = 1 << 3;
pub const PAD_TRIGGER_Z: u16 = 1 << 4;
pub const PAD_TRIGGER_R: u16 = 1 << 5;
pub const PAD_TRIGGER_L: u16 = 1 << 6;
pub const PAD_BUTTON_A: u16 = 1 << 8;
pub const PAD_BUTTON_B: u16 = 1 << 9;
pub const PAD_BUTTON_X: u16 = 1 << 10;
pub const PAD_BUTTON_Y: u16 = 1 << 11;
pub const PAD_BUTTON_START: u16 = 1 << 12;

pub const PAD_ERR_NONE: i8 = 0;
pub const PAD_ERR_NO_CONTROLLER: i8 = -1;
pub const PAD_ERR_NOT_READY: i8 = -2;
pub const PAD_ERR_TRANSFER: i8 = -3;

pub const PAD_MOTOR_STOP: u32 = 0;
pub const PAD_MOTOR_RUMBLE: u32 = 1;
pub const PAD_MOTOR_STOP_HARD: u32 = 2;

pub const PAD_CHAN0_BIT: u32 = 0x8000_0000;
pub const STICK_GATE: i32 = 80;
pub const STICK_DEAD: i32 = 15;
pub const TRIGGER_DEAD: u8 = 30;
pub const TRIGGER_MAX: u8 = 180;

/// Exact layout of Dolphin SDK `PADStatus` (12 bytes with tail padding).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PadStatus {
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
    pub substick_x: i8,
    pub substick_y: i8,
    pub trigger_left: u8,
    pub trigger_right: u8,
    pub analog_a: u8,
    pub analog_b: u8,
    pub err: i8,
}

impl Default for PadStatus {
    fn default() -> Self {
        Self {
            button: 0,
            stick_x: 0,
            stick_y: 0,
            substick_x: 0,
            substick_y: 0,
            trigger_left: 0,
            trigger_right: 0,
            analog_a: 0,
            analog_b: 0,
            err: PAD_ERR_NO_CONTROLLER,
        }
    }
}

impl PadStatus {
    pub fn connected(self) -> Self {
        let mut s = self;
        s.err = PAD_ERR_NONE;
        s
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RawPad {
    pub connected: bool,
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
    pub substick_x: i8,
    pub substick_y: i8,
    pub trigger_left: u8,
    pub trigger_right: u8,
    pub analog_a: u8,
    pub analog_b: u8,
    pub name: PadKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadKind {
    #[default]
    Empty,
    Keyboard,
    GcAdapter,
    Xbox,
    DualShock,
    SwitchPro,
    Generic,
}

impl PadKind {
    pub fn label(self) -> &'static str {
        match self {
            PadKind::Empty => "empty",
            PadKind::Keyboard => "keyboard",
            PadKind::GcAdapter => "gcc-adapter",
            PadKind::Xbox => "xbox",
            PadKind::DualShock => "dualshock",
            PadKind::SwitchPro => "switch-pro",
            PadKind::Generic => "generic",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct KeyboardState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub t: bool,
    pub f: bool,
    pub g: bool,
    pub h: bool,
    pub space: bool,
    pub shift: bool,
    pub x: bool,
    pub c: bool,
    pub q: bool,
    pub e: bool,
    pub z: bool,
    pub enter: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl KeyboardState {
    pub fn to_raw(self) -> RawPad {
        let mut button = 0u16;
        if self.space {
            button |= PAD_BUTTON_A;
        }
        if self.shift {
            button |= PAD_BUTTON_B;
        }
        if self.x {
            button |= PAD_BUTTON_X;
        }
        if self.c {
            button |= PAD_BUTTON_Y;
        }
        if self.z {
            button |= PAD_TRIGGER_Z;
        }
        if self.q {
            button |= PAD_TRIGGER_L;
        }
        if self.e {
            button |= PAD_TRIGGER_R;
        }
        if self.enter {
            button |= PAD_BUTTON_START;
        }
        if self.left {
            button |= PAD_BUTTON_LEFT;
        }
        if self.right {
            button |= PAD_BUTTON_RIGHT;
        }
        if self.down {
            button |= PAD_BUTTON_DOWN;
        }
        if self.up {
            button |= PAD_BUTTON_UP;
        }
        RawPad {
            connected: true,
            button,
            stick_x: axis(self.d, self.a),
            stick_y: axis(self.w, self.s),
            substick_x: axis(self.h, self.f),
            substick_y: axis(self.t, self.g),
            trigger_left: if self.q { 255 } else { 0 },
            trigger_right: if self.e { 255 } else { 0 },
            analog_a: 0,
            analog_b: 0,
            name: PadKind::Keyboard,
        }
    }
}

fn axis(pos: bool, neg: bool) -> i8 {
    match (pos, neg) {
        (true, false) => STICK_GATE as i8,
        (false, true) => -(STICK_GATE as i8),
        _ => 0,
    }
}

/// Linux `struct js_event` (8 bytes).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct JsEvent {
    time: u32,
    value: i16,
    kind: u8,
    number: u8,
}

const JS_EVENT_BUTTON: u8 = 0x01;
const JS_EVENT_AXIS: u8 = 0x02;
const JS_EVENT_INIT: u8 = 0x80;

struct JsDevice {
    file: File,
    path: PathBuf,
    kind: PadKind,
    buttons: [bool; 24],
    axes: [i16; 8],
}

impl JsDevice {
    fn open(path: PathBuf) -> Option<Self> {
        let file = OpenOptions::new().read(true).write(true).open(&path).ok()?;
        let fd = file.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL, 0);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        let kind = classify_js(&path);
        Some(Self {
            file,
            path,
            kind,
            buttons: [false; 24],
            axes: [0; 8],
        })
    }

    fn poll(&mut self) {
        loop {
            let mut buf = [0u8; 8];
            match self.file.read(&mut buf) {
                Ok(8) => {
                    let ev = JsEvent {
                        time: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
                        value: i16::from_le_bytes(buf[4..6].try_into().unwrap()),
                        kind: buf[6],
                        number: buf[7],
                    };
                    let kind = ev.kind & !JS_EVENT_INIT;
                    if kind == JS_EVENT_BUTTON {
                        if let Some(slot) = self.buttons.get_mut(ev.number as usize) {
                            *slot = ev.value != 0;
                        }
                    } else if kind == JS_EVENT_AXIS {
                        if let Some(slot) = self.axes.get_mut(ev.number as usize) {
                            *slot = ev.value;
                        }
                    }
                }
                Ok(_) | Err(_) => break,
            }
        }
    }

    fn to_raw(&self) -> RawPad {
        map_js(self.kind, &self.buttons, &self.axes)
    }
}

fn classify_js(path: &std::path::Path) -> PadKind {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let sys = format!("/sys/class/input/{name}/device/name");
    let label = fs::read_to_string(sys)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if label.contains("xbox") || label.contains("x-box") || label.contains("microsoft") {
        PadKind::Xbox
    } else if label.contains("dualshock")
        || label.contains("wireless controller")
        || label.contains("dualsense")
        || label.contains("sony")
    {
        PadKind::DualShock
    } else if label.contains("pro controller") || label.contains("nintendo") {
        PadKind::SwitchPro
    } else if label.contains("gamecube") || label.contains("mayflash") || label.contains("gcc") {
        PadKind::GcAdapter
    } else {
        PadKind::Generic
    }
}

fn axis_i8(v: i16) -> i8 {
    let scaled = (v as i32 * 127) / 32767;
    scaled.clamp(-128, 127) as i8
}

fn trigger_u8(v: i16) -> u8 {
    if v < 0 {
        (((v as i32) + 32767) * 255 / 65534).clamp(0, 255) as u8
    } else {
        (v as i32 * 255 / 32767).clamp(0, 255) as u8
    }
}

fn map_js(kind: PadKind, buttons: &[bool; 24], axes: &[i16; 8]) -> RawPad {
    let mut button = 0u16;
    let a = buttons[0];
    let b = buttons[1];
    let x = buttons[2];
    let y = buttons[3];
    let lb = buttons[4];
    let rb = buttons[5];
    let start = buttons[7] || buttons[9];
    if a { button |= PAD_BUTTON_A; }
    if b { button |= PAD_BUTTON_B; }
    if x { button |= PAD_BUTTON_X; }
    if y { button |= PAD_BUTTON_Y; }
    if rb { button |= PAD_TRIGGER_Z; }
    if lb { button |= PAD_TRIGGER_L; }
    if start { button |= PAD_BUTTON_START; }
    let hat_x = axes.get(6).copied().unwrap_or(0);
    let hat_y = axes.get(7).copied().unwrap_or(0);
    if hat_x < -1000 || buttons.get(11).copied().unwrap_or(false) { button |= PAD_BUTTON_LEFT; }
    if hat_x > 1000 || buttons.get(12).copied().unwrap_or(false) { button |= PAD_BUTTON_RIGHT; }
    if hat_y < -1000 || buttons.get(13).copied().unwrap_or(false) { button |= PAD_BUTTON_UP; }
    if hat_y > 1000 || buttons.get(14).copied().unwrap_or(false) { button |= PAD_BUTTON_DOWN; }
    let mut trig_l = trigger_u8(axes[2]);
    let mut trig_r = trigger_u8(axes[5].max(axes[4]));
    if kind == PadKind::Xbox {
        trig_l = trigger_u8(axes[2]);
        trig_r = trigger_u8(axes[5]);
    }
    if lb { trig_l = trig_l.max(200); }
    if trig_l > 200 { button |= PAD_TRIGGER_L; }
    if trig_r > 200 { button |= PAD_TRIGGER_R; }
    RawPad {
        connected: true,
        button,
        stick_x: axis_i8(axes[0]),
        stick_y: axis_i8(-axes[1]),
        substick_x: axis_i8(axes[3].max(axes[2])),
        substick_y: axis_i8(-axes.get(4).copied().unwrap_or(0)),
        trigger_left: trig_l,
        trigger_right: trig_r,
        analog_a: 0,
        analog_b: 0,
        name: kind,
    }
}

struct GcAdapter {
    file: File,
    rumble: [u8; 4],
    last: [RawPad; 4],
    last_ok: Instant,
}

impl GcAdapter {
    fn discover() -> Option<Self> {
        let entries = fs::read_dir("/sys/class/hidraw").ok()?;
        for ent in entries.flatten() {
            let name = ent.file_name();
            let uevent = ent.path().join("device/uevent");
            let text = fs::read_to_string(uevent).unwrap_or_default();
            let is_adapter = text.to_ascii_lowercase().contains("0000057e:00000337")
                || text.contains("HID_ID=0003:0000057E:00000337");
            if !is_adapter { continue; }
            let dev = PathBuf::from("/dev").join(&name);
            if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&dev) {
                let fd = file.as_raw_fd();
                unsafe {
                    let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                let _ = file.write_all(&[0x13]);
                log::info!("opened Wii U GameCube adapter at {}", dev.display());
                return Some(Self { file, rumble: [0; 4], last: [RawPad::default(); 4], last_ok: Instant::now() });
            }
        }
        None
    }

    fn poll(&mut self) {
        let mut buf = [0u8; 37];
        match self.file.read(&mut buf) {
            Ok(n) if n >= 37 || (n >= 1 && buf[0] == 0x21) => {
                self.decode(&buf);
                self.last_ok = Instant::now();
            }
            _ => {
                if self.last_ok.elapsed() > Duration::from_millis(500) {
                    self.last = [RawPad::default(); 4];
                }
            }
        }
    }

    fn decode(&mut self, buf: &[u8]) {
        let payload = if buf[0] == 0x21 { &buf[1..] } else { buf };
        for port in 0..4 {
            let o = port * 9;
            if o + 8 >= payload.len() { self.last[port] = RawPad::default(); continue; }
            let ty = payload[o];
            if ty == 0 { self.last[port] = RawPad::default(); continue; }
            let b1 = payload[o + 1];
            let b2 = payload[o + 2];
            let mut button = 0u16;
            if b1 & 0x01 != 0 { button |= PAD_BUTTON_A; }
            if b1 & 0x02 != 0 { button |= PAD_BUTTON_B; }
            if b1 & 0x04 != 0 { button |= PAD_BUTTON_X; }
            if b1 & 0x08 != 0 { button |= PAD_BUTTON_Y; }
            if b1 & 0x10 != 0 { button |= PAD_BUTTON_LEFT; }
            if b1 & 0x20 != 0 { button |= PAD_BUTTON_RIGHT; }
            if b1 & 0x40 != 0 { button |= PAD_BUTTON_DOWN; }
            if b1 & 0x80 != 0 { button |= PAD_BUTTON_UP; }
            if b2 & 0x01 != 0 { button |= PAD_BUTTON_START; }
            if b2 & 0x02 != 0 { button |= PAD_TRIGGER_Z; }
            if b2 & 0x04 != 0 { button |= PAD_TRIGGER_R; }
            if b2 & 0x08 != 0 { button |= PAD_TRIGGER_L; }
            self.last[port] = RawPad {
                connected: true,
                button,
                stick_x: (payload[o + 3] as i16 - 128) as i8,
                stick_y: (payload[o + 4] as i16 - 128) as i8,
                substick_x: (payload[o + 5] as i16 - 128) as i8,
                substick_y: (payload[o + 6] as i16 - 128) as i8,
                trigger_left: payload[o + 7],
                trigger_right: payload[o + 8],
                analog_a: 0,
                analog_b: 0,
                name: PadKind::GcAdapter,
            };
        }
    }

    fn set_rumble(&mut self, chan: usize, on: bool) {
        if chan < 4 {
            self.rumble[chan] = if on { 1 } else { 0 };
            let pkt = [0x11, self.rumble[0], self.rumble[1], self.rumble[2], self.rumble[3]];
            let _ = self.file.write_all(&pkt);
        }
    }
}

pub struct PadSystem {
    keyboard: KeyboardState,
    adapter: Option<GcAdapter>,
    sticks: Vec<JsDevice>,
    rumble: [u32; 4],
    last: [PadStatus; 4],
    kinds: [PadKind; 4],
}

impl PadSystem {
    pub fn new() -> Self {
        let adapter = GcAdapter::discover();
        let mut sticks = Vec::new();
        if let Ok(dir) = fs::read_dir("/dev/input") {
            for ent in dir.flatten() {
                let name = ent.file_name();
                if let Some(s) = name.to_str() {
                    if s.starts_with("js") {
                        if let Some(dev) = JsDevice::open(ent.path()) {
                            log::info!("opened joystick {} ({})", ent.path().display(), dev.kind.label());
                            sticks.push(dev);
                        }
                    }
                }
            }
        }
        Self {
            keyboard: KeyboardState::default(),
            adapter,
            sticks,
            rumble: [PAD_MOTOR_STOP; 4],
            last: [PadStatus::default(); 4],
            kinds: [PadKind::Empty; 4],
        }
    }

    pub fn keyboard_mut(&mut self) -> &mut KeyboardState { &mut self.keyboard }
    pub fn kinds(&self) -> [PadKind; 4] { self.kinds }
    pub fn last(&self) -> [PadStatus; 4] { self.last }

    pub fn set_motor(&mut self, chan: i32, command: u32) {
        if (0..4).contains(&chan) {
            self.rumble[chan as usize] = command;
            if let Some(adapter) = self.adapter.as_mut() {
                adapter.set_rumble(chan as usize, command == PAD_MOTOR_RUMBLE);
            }
        }
    }

    pub fn poll(&mut self) -> [PadStatus; 4] {
        if let Some(adapter) = self.adapter.as_mut() { adapter.poll(); }
        for js in &mut self.sticks { js.poll(); }
        let mut raw = [RawPad::default(); 4];
        let mut used_js = 0usize;
        if let Some(adapter) = self.adapter.as_ref() { raw = adapter.last; }
        for port in 0..4 {
            if raw[port].connected { continue; }
            if let Some(js) = self.sticks.get(used_js) {
                raw[port] = js.to_raw();
                used_js += 1;
            }
        }
        if !raw[0].connected { raw[0] = self.keyboard.to_raw(); }
        let mut out = [PadStatus::default(); 4];
        for i in 0..4 {
            self.kinds[i] = raw[i].name;
            if !raw[i].connected { out[i] = PadStatus::default(); continue; }
            let mut st = PadStatus {
                button: raw[i].button,
                stick_x: raw[i].stick_x,
                stick_y: raw[i].stick_y,
                substick_x: raw[i].substick_x,
                substick_y: raw[i].substick_y,
                trigger_left: raw[i].trigger_left,
                trigger_right: raw[i].trigger_right,
                analog_a: raw[i].analog_a,
                analog_b: raw[i].analog_b,
                err: PAD_ERR_NONE,
            };
            clamp_status(&mut st);
            out[i] = st;
        }
        self.last = out;
        out
    }
}

impl Default for PadSystem {
    fn default() -> Self { Self::new() }
}

pub fn clamp_status(st: &mut PadStatus) {
    clamp_stick(&mut st.stick_x, &mut st.stick_y, STICK_GATE, STICK_DEAD);
    clamp_stick(&mut st.substick_x, &mut st.substick_y, STICK_GATE, STICK_DEAD);
    st.trigger_left = clamp_trigger(st.trigger_left);
    st.trigger_right = clamp_trigger(st.trigger_right);
}

fn clamp_stick(px: &mut i8, py: &mut i8, max: i32, dead: i32) {
    let mut x = *px as i32;
    let mut y = *py as i32;
    if x.abs() <= dead && y.abs() <= dead { *px = 0; *py = 0; return; }
    let mag2 = x * x + y * y;
    let max2 = max * max;
    if mag2 > max2 {
        let mag = (mag2 as f64).sqrt();
        x = ((x as f64) * (max as f64) / mag).round() as i32;
        y = ((y as f64) * (max as f64) / mag).round() as i32;
        while x * x + y * y > max2 {
            if x.abs() >= y.abs() { x -= x.signum(); } else { y -= y.signum(); }
        }
    }
    *px = x.clamp(-128, 127) as i8;
    *py = y.clamp(-128, 127) as i8;
}

fn clamp_trigger(v: u8) -> u8 {
    if v < TRIGGER_DEAD { 0 }
    else if v > TRIGGER_MAX { 255 }
    else {
        let span = (TRIGGER_MAX - TRIGGER_DEAD) as u16;
        (((v - TRIGGER_DEAD) as u16) * 255 / span) as u8
    }
}

pub fn clamp_slice(status: &mut [PadStatus]) {
    for st in status.iter_mut() {
        if st.err == PAD_ERR_NONE { clamp_status(st); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadzone_centers() {
        let mut st = PadStatus { stick_x: 10, stick_y: -8, err: PAD_ERR_NONE, ..PadStatus::default() };
        clamp_status(&mut st);
        assert_eq!(st.stick_x, 0);
        assert_eq!(st.stick_y, 0);
    }
    #[test]
    fn gate_clamps_diagonal() {
        let mut st = PadStatus { stick_x: 127, stick_y: 127, err: PAD_ERR_NONE, ..PadStatus::default() };
        clamp_status(&mut st);
        let mag = (st.stick_x as i32).pow(2) + (st.stick_y as i32).pow(2);
        assert!(mag <= STICK_GATE * STICK_GATE + 4);
    }
    #[test]
    fn keyboard_a_button() {
        let mut kb = KeyboardState::default();
        kb.space = true;
        kb.w = true;
        let raw = kb.to_raw();
        assert_eq!(raw.button & PAD_BUTTON_A, PAD_BUTTON_A);
        assert_eq!(raw.stick_y, STICK_GATE as i8);
    }
    #[test]
    fn sizeof_padstatus_matches_sdk() {
        assert_eq!(std::mem::size_of::<PadStatus>(), 12);
    }
    #[test]
    fn gcc_bit_layout() {
        assert_eq!(PAD_BUTTON_A, 0x0100);
        assert_eq!(PAD_BUTTON_START, 0x1000);
        assert_eq!(PAD_TRIGGER_Z, 0x0010);
    }
}
