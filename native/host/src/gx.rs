//! GX command-processor stand-in.
//!
//! Records immediate-mode GX into a host draw list that the wgpu Vulkan
//! renderer consumes at `GXCopyDisp` / frame present. Enough of XF + TEV +
//! pixel state is modelled to drive HSD JObj/DObj/PObj once the decomp is
//! compiled against this crate.

use glam::{Mat4, Vec3, Vec4};

pub const GX_TRIANGLES: u8 = 0x90;
pub const GX_TRIANGLESTRIP: u8 = 0x98;
pub const GX_TRIANGLEFAN: u8 = 0xA0;
pub const GX_QUADS: u8 = 0x80;
pub const GX_LINES: u8 = 0xA8;
pub const GX_POINTS: u8 = 0xB8;

pub const GX_MODULATE: u8 = 0;
pub const GX_DECAL: u8 = 1;
pub const GX_REPLACE: u8 = 3;
pub const GX_PASSCLR: u8 = 4;
pub const GX_BLEND: u8 = 2;

pub const GX_CULL_NONE: u8 = 0;
pub const GX_CULL_BACK: u8 = 2;
pub const GX_CULL_FRONT: u8 = 1;
pub const GX_CULL_ALL: u8 = 3;

pub const GX_BM_NONE: u8 = 0;
pub const GX_BM_BLEND: u8 = 1;
pub const GX_BM_LOGIC: u8 = 2;
pub const GX_BM_SUBTRACT: u8 = 3;

pub const GX_BL_ZERO: u8 = 0;
pub const GX_BL_ONE: u8 = 1;
pub const GX_BL_SRCCLR: u8 = 2;
pub const GX_BL_SRCALPHA: u8 = 4;
pub const GX_BL_INVSRCALPHA: u8 = 5;
pub const GX_BL_DSTALPHA: u8 = 6;

pub const GX_ALWAYS: u8 = 7;
pub const GX_LEQUAL: u8 = 3;
pub const GX_GEQUAL: u8 = 6;
pub const GX_EQUAL: u8 = 2;
pub const GX_NEVER: u8 = 0;

pub const GX_PERSPECTIVE: u8 = 0;
pub const GX_ORTHOGRAPHIC: u8 = 1;

pub const GX_PNMTX0: u32 = 0;
pub const GX_TEXCOORD0: u8 = 0;
pub const GX_TEXMAP0: u8 = 0;
pub const GX_COLOR0A0: u8 = 0;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            pos: [0.0; 3],
            nrm: [0.0, 0.0, 1.0],
            color: [1.0; 4],
            uv: [0.0; 2],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PipelineState {
    pub tev: u8,
    pub cull: u8,
    pub blend: u8,
    pub src_factor: u8,
    pub dst_factor: u8,
    pub z_enable: bool,
    pub z_write: bool,
    pub z_func: u8,
    pub alpha_ref: u8,
    pub num_tev: u8,
    pub tex_enabled: bool,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            tev: GX_MODULATE,
            cull: GX_CULL_BACK,
            blend: GX_BM_NONE,
            src_factor: GX_BL_SRCALPHA,
            dst_factor: GX_BL_INVSRCALPHA,
            z_enable: true,
            z_write: true,
            z_func: GX_LEQUAL,
            alpha_ref: 0,
            num_tev: 1,
            tex_enabled: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawCmd {
    pub verts: Vec<Vertex>,
    pub topology: u8,
    pub pipeline: PipelineState,
    pub mvp: Mat4,
    pub texture: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub n: f32,
    pub f: f32,
}

pub struct GxState {
    pub projection: Mat4,
    pub pos_mtx: Mat4,
    pub viewport: Viewport,
    pub scissor: (u32, u32, u32, u32),
    pub pipeline: PipelineState,
    pub current: Vertex,
    pub prim: u8,
    pub expected: u16,
    pub assembling: Vec<Vertex>,
    pub cmds: Vec<DrawCmd>,
    pub clear: [f32; 4],
    pub clear_z: f32,
    pub bound_tex: Option<u32>,
    pub textures: Vec<HostTexture>,
    pub vtx_has_nrm: bool,
    pub vtx_has_clr: bool,
    pub vtx_has_tex: bool,
}

pub struct HostTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Default for GxState {
    fn default() -> Self { Self::new() }
}

impl GxState {
    pub fn new() -> Self {
        Self {
            projection: Mat4::IDENTITY,
            pos_mtx: Mat4::IDENTITY,
            viewport: Viewport { x: 0.0, y: 0.0, w: 640.0, h: 480.0, n: 0.0, f: 1.0 },
            scissor: (0, 0, 640, 480),
            pipeline: PipelineState::default(),
            current: Vertex::default(),
            prim: GX_TRIANGLES,
            expected: 0,
            assembling: Vec::new(),
            cmds: Vec::new(),
            clear: [0.05, 0.06, 0.10, 1.0],
            clear_z: 1.0,
            bound_tex: None,
            textures: Vec::new(),
            vtx_has_nrm: false,
            vtx_has_clr: true,
            vtx_has_tex: false,
        }
    }

    pub fn init(&mut self) { *self = Self::new(); }
    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, n: f32, f: f32) {
        self.viewport = Viewport { x, y, w, h, n, f };
    }
    pub fn set_scissor(&mut self, x: u32, y: u32, w: u32, h: u32) { self.scissor = (x, y, w, h); }
    pub fn set_projection(&mut self, mtx: &[f32; 16], _kind: u8) {
        self.projection = Mat4::from_cols_array(mtx);
    }
    pub fn load_pos_mtx(&mut self, mtx: &[f32], _id: u32) {
        let mut cols = [0.0f32; 16];
        if mtx.len() >= 12 {
            cols[0] = mtx[0]; cols[4] = mtx[1]; cols[8] = mtx[2]; cols[12] = mtx[3];
            cols[1] = mtx[4]; cols[5] = mtx[5]; cols[9] = mtx[6]; cols[13] = mtx[7];
            cols[2] = mtx[8]; cols[6] = mtx[9]; cols[10] = mtx[10]; cols[14] = mtx[11];
            cols[15] = 1.0;
            self.pos_mtx = Mat4::from_cols_array(&cols);
        } else if mtx.len() >= 16 {
            let mut a = [0.0; 16];
            a.copy_from_slice(&mtx[..16]);
            self.pos_mtx = Mat4::from_cols_array(&a);
        }
    }
    pub fn set_cull(&mut self, mode: u8) { self.pipeline.cull = mode; }
    pub fn set_blend(&mut self, kind: u8, src: u8, dst: u8, _logic: u8) {
        self.pipeline.blend = kind;
        self.pipeline.src_factor = src;
        self.pipeline.dst_factor = dst;
    }
    pub fn set_z_mode(&mut self, enable: bool, func: u8, write: bool) {
        self.pipeline.z_enable = enable;
        self.pipeline.z_func = func;
        self.pipeline.z_write = write;
    }
    pub fn set_tev_op(&mut self, _stage: u8, op: u8) { self.pipeline.tev = op; }
    pub fn set_num_tev(&mut self, n: u8) { self.pipeline.num_tev = n.max(1); }
    pub fn set_chan_ctrl(&mut self, _chan: u8, enable: bool) { self.vtx_has_clr = enable; }
    pub fn copy_clear(&mut self, r: u8, g: u8, b: u8, a: u8, z: f32) {
        self.clear = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
        self.clear_z = z;
    }
    pub fn begin(&mut self, prim: u8, _fmt: u8, nverts: u16) {
        self.prim = prim;
        self.expected = nverts;
        self.assembling.clear();
        self.assembling.reserve(nverts as usize);
        self.current = Vertex::default();
    }
    pub fn position3f32(&mut self, x: f32, y: f32, z: f32) {
        self.current.pos = [x, y, z];
        self.push_vertex();
    }
    pub fn position3s16(&mut self, x: i16, y: i16, z: i16) {
        self.position3f32(x as f32, y as f32, z as f32);
    }
    pub fn normal3f32(&mut self, x: f32, y: f32, z: f32) {
        self.current.nrm = [x, y, z];
        self.vtx_has_nrm = true;
    }
    pub fn color4u8(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.current.color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
        self.vtx_has_clr = true;
    }
    pub fn color1u32(&mut self, rgba: u32) {
        self.color4u8(((rgba >> 24) & 0xff) as u8, ((rgba >> 16) & 0xff) as u8, ((rgba >> 8) & 0xff) as u8, (rgba & 0xff) as u8);
    }
    pub fn texcoord2f32(&mut self, u: f32, v: f32) {
        self.current.uv = [u, v];
        self.vtx_has_tex = true;
        self.pipeline.tex_enabled = true;
    }
    fn push_vertex(&mut self) {
        self.assembling.push(self.current);
        if self.expected > 0 && self.assembling.len() >= self.expected as usize { self.end(); }
    }
    pub fn end(&mut self) {
        if self.assembling.is_empty() { return; }
        let verts = match self.prim {
            GX_QUADS => quads_to_tris(&self.assembling),
            GX_TRIANGLEFAN => fan_to_tris(&self.assembling),
            _ => self.assembling.clone(),
        };
        let topology = if self.prim == GX_QUADS || self.prim == GX_TRIANGLEFAN { GX_TRIANGLES } else { self.prim };
        let mvp = self.projection * self.pos_mtx;
        self.cmds.push(DrawCmd { verts, topology, pipeline: self.pipeline, mvp, texture: self.bound_tex });
        self.assembling.clear();
        self.expected = 0;
    }
    pub fn abort_frame(&mut self) { self.cmds.clear(); self.assembling.clear(); }
    pub fn take_cmds(&mut self) -> Vec<DrawCmd> { self.end(); std::mem::take(&mut self.cmds) }
    pub fn create_texture_rgba(&mut self, width: u32, height: u32, rgba: Vec<u8>) -> u32 {
        let id = self.textures.len() as u32;
        self.textures.push(HostTexture { width, height, rgba });
        id
    }
    pub fn bind_texture(&mut self, id: u32) { self.bound_tex = Some(id); self.pipeline.tex_enabled = true; }
    pub fn perspective(&mut self, fov_y_deg: f32, aspect: f32, n: f32, f: f32) {
        self.projection = Mat4::perspective_rh(fov_y_deg.to_radians(), aspect, n, f);
    }
    pub fn ortho(&mut self, l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) {
        self.projection = Mat4::orthographic_rh(l, r, b, t, n, f);
    }
    pub fn look_at(&mut self, eye: Vec3, target: Vec3, up: Vec3) {
        self.pos_mtx = Mat4::look_at_rh(eye, target, up);
    }
}

fn quads_to_tris(v: &[Vertex]) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(v.len() / 4 * 6);
    for chunk in v.chunks(4) {
        if chunk.len() < 4 { break; }
        out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], chunk[0], chunk[2], chunk[3]]);
    }
    out
}
fn fan_to_tris(v: &[Vertex]) -> Vec<Vertex> {
    if v.len() < 3 { return v.to_vec(); }
    let mut out = Vec::with_capacity((v.len() - 2) * 3);
    for i in 1..v.len() - 1 { out.extend_from_slice(&[v[0], v[i], v[i + 1]]); }
    out
}
pub fn mtx_perspective(fov_y_deg: f32, aspect: f32, n: f32, f: f32) -> [f32; 16] {
    Mat4::perspective_rh(fov_y_deg.to_radians(), aspect, n, f).to_cols_array()
}
pub fn mtx_identity() -> [f32; 12] {
    [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]
}
pub fn mtx_concat(a: &[f32; 12], b: &[f32; 12]) -> [f32; 12] {
    let mut o = [0.0f32; 12];
    for r in 0..3 {
        for c in 0..4 {
            let mut s = 0.0;
            for k in 0..3 { s += a[r * 4 + k] * b[k * 4 + c]; }
            if c == 3 { s += a[r * 4 + 3]; }
            o[r * 4 + c] = s;
        }
    }
    o
}
pub fn decode_tex(fmt: u8, width: u32, height: u32, src: &[u8]) -> Vec<u8> {
    match fmt {
        6 => decode_rgba8(width, height, src),
        4 => decode_rgb565(width, height, src),
        1 => decode_i8(width, height, src),
        3 => decode_ia8(width, height, src),
        0xE => decode_cmpr(width, height, src),
        _ => vec![180, 40, 200, 255].repeat((width * height) as usize),
    }
}
fn decode_i8(w: u32, h: u32, src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        let v = src.get(i).copied().unwrap_or(255);
        let o = i * 4; out[o] = v; out[o + 1] = v; out[o + 2] = v; out[o + 3] = 255;
    }
    out
}
fn decode_ia8(w: u32, h: u32, src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        let a = src.get(i * 2).copied().unwrap_or(255);
        let v = src.get(i * 2 + 1).copied().unwrap_or(255);
        let o = i * 4; out[o] = v; out[o + 1] = v; out[o + 2] = v; out[o + 3] = a;
    }
    out
}
fn decode_rgb565(w: u32, h: u32, src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        let hi = src.get(i * 2).copied().unwrap_or(0) as u16;
        let lo = src.get(i * 2 + 1).copied().unwrap_or(0) as u16;
        let p = (hi << 8) | lo;
        let o = i * 4;
        out[o] = (((p >> 11) & 0x1f) * 255 / 31) as u8;
        out[o + 1] = (((p >> 5) & 0x3f) * 255 / 63) as u8;
        out[o + 2] = ((p & 0x1f) * 255 / 31) as u8;
        out[o + 3] = 255;
    }
    out
}
fn decode_rgba8(w: u32, h: u32, src: &[u8]) -> Vec<u8> {
    if src.len() >= (w * h * 4) as usize && src.len() % 64 != 0 {
        return src[..(w * h * 4) as usize].to_vec();
    }
    let mut out = vec![0u8; (w * h * 4) as usize];
    let mut src_off = 0usize;
    for ty in 0..(h / 4).max(1) {
        for tx in 0..(w / 4).max(1) {
            let mut ar = [0u8; 32];
            for i in 0..32 { ar[i] = src.get(src_off + i).copied().unwrap_or(0); }
            src_off += 32;
            let mut gb = [0u8; 32];
            for i in 0..32 { gb[i] = src.get(src_off + i).copied().unwrap_or(0); }
            src_off += 32;
            for py in 0..4 {
                for px in 0..4 {
                    let i = py * 4 + px;
                    let x = tx * 4 + px as u32;
                    let y = ty * 4 + py as u32;
                    if x >= w || y >= h { continue; }
                    let o = ((y * w + x) * 4) as usize;
                    out[o] = ar[i * 2 + 1]; out[o + 1] = gb[i * 2]; out[o + 2] = gb[i * 2 + 1]; out[o + 3] = ar[i * 2];
                }
            }
        }
    }
    out
}
fn decode_cmpr(w: u32, h: u32, src: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    let mut off = 0usize;
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            for ty in 0..2 {
                for tx in 0..2 {
                    let block = &src.get(off..off + 8).unwrap_or(&[0; 8][..]);
                    off += 8;
                    decode_dxt1_block(block, &mut out, w, bx + tx * 4, by + ty * 4);
                }
            }
        }
    }
    out
}
fn decode_dxt1_block(block: &[u8], out: &mut [u8], w: u32, x0: u32, y0: u32) {
    if block.len() < 8 { return; }
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let idx = u32::from_le_bytes(block[4..8].try_into().unwrap_or([0; 4]));
    let rgb = |c: u16| -> [u8; 3] {
        [((c >> 11) * 255 / 31) as u8, (((c >> 5) & 0x3f) * 255 / 63) as u8, ((c & 0x1f) * 255 / 31) as u8]
    };
    let a = rgb(c0); let b = rgb(c1);
    let cols = if c0 > c1 {
        [a, b,
         [((2 * a[0] as u16 + b[0] as u16) / 3) as u8, ((2 * a[1] as u16 + b[1] as u16) / 3) as u8, ((2 * a[2] as u16 + b[2] as u16) / 3) as u8],
         [((a[0] as u16 + 2 * b[0] as u16) / 3) as u8, ((a[1] as u16 + 2 * b[1] as u16) / 3) as u8, ((a[2] as u16 + 2 * b[2] as u16) / 3) as u8]]
    } else {
        [a, b,
         [((a[0] as u16 + b[0] as u16) / 2) as u8, ((a[1] as u16 + b[1] as u16) / 2) as u8, ((a[2] as u16 + b[2] as u16) / 2) as u8],
         [0, 0, 0]]
    };
    for py in 0..4 {
        for px in 0..4 {
            let i = py * 4 + px;
            let ci = ((idx >> (i * 2)) & 3) as usize;
            let x = x0 + px; let y = y0 + py;
            if x >= w { continue; }
            let o = ((y * w + x) * 4) as usize;
            if o + 3 >= out.len() { continue; }
            out[o] = cols[ci][0]; out[o + 1] = cols[ci][1]; out[o + 2] = cols[ci][2];
            out[o + 3] = if c0 <= c1 && ci == 3 { 0 } else { 255 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quads_become_two_tris() {
        let mut gx = GxState::new();
        gx.begin(GX_QUADS, 0, 4);
        gx.color4u8(255, 0, 0, 255);
        gx.position3f32(-1.0, -1.0, 0.0);
        gx.position3f32(1.0, -1.0, 0.0);
        gx.position3f32(1.0, 1.0, 0.0);
        gx.position3f32(-1.0, 1.0, 0.0);
        let cmds = gx.take_cmds();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].verts.len(), 6);
        assert_eq!(cmds[0].topology, GX_TRIANGLES);
    }
    #[test]
    fn mtx_concat_identity() {
        let i = mtx_identity();
        assert_eq!(mtx_concat(&i, &i), i);
    }
}
