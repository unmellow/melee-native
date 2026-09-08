//! Host window + 60 Hz game loop. Draws a GX demo (platform + 4-port HUD).

use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::gx::{GxState, GX_BM_BLEND, GX_CULL_NONE, GX_LINES, GX_MODULATE, GX_QUADS, GX_TRIANGLES};
use crate::pad::{
    PadSystem, PAD_BUTTON_A, PAD_BUTTON_B, PAD_BUTTON_START, PAD_BUTTON_X, PAD_BUTTON_Y,
    PAD_TRIGGER_L, PAD_TRIGGER_R, PAD_TRIGGER_Z,
};
use crate::renderer::Renderer;
use crate::{dvd, os};

pub struct HostApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gx: GxState,
    pad: PadSystem,
    start: Instant,
    frames: u64,
    headless: bool,
}

impl HostApp {
    pub fn new(headless: bool) -> Self {
        os::init();
        dvd::init();
        Self {
            window: None,
            renderer: None,
            gx: GxState::new(),
            pad: PadSystem::new(),
            start: Instant::now(),
            frames: 0,
            headless,
        }
    }

    fn frame(&mut self) {
        let pads = self.pad.poll();
        self.gx.abort_frame();
        draw_demo(
            &mut self.gx,
            &pads,
            self.frames,
            self.start.elapsed().as_secs_f32(),
        );
        if let Some(r) = self.renderer.as_mut() {
            match r.render(&mut self.gx) {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    r.resize(r.size);
                }
                Err(err) => log::error!("present: {err}"),
            }
        }
        self.frames += 1;
    }
}

impl ApplicationHandler for HostApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Melee Native — Vulkan / wgpu")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 960.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        if !self.headless {
            self.renderer = Some(Renderer::new(window.clone()));
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(size);
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                let down = state == ElementState::Pressed;
                if down && code == KeyCode::Escape {
                    event_loop.exit();
                    return;
                }
                let kb = self.pad.keyboard_mut();
                match code {
                    KeyCode::KeyW => kb.w = down,
                    KeyCode::KeyA => kb.a = down,
                    KeyCode::KeyS => kb.s = down,
                    KeyCode::KeyD => kb.d = down,
                    KeyCode::KeyT => kb.t = down,
                    KeyCode::KeyF => kb.f = down,
                    KeyCode::KeyG => kb.g = down,
                    KeyCode::KeyH => kb.h = down,
                    KeyCode::Space => kb.space = down,
                    KeyCode::ShiftLeft => kb.shift = down,
                    KeyCode::KeyX => kb.x = down,
                    KeyCode::KeyC => kb.c = down,
                    KeyCode::KeyQ => kb.q = down,
                    KeyCode::KeyE => kb.e = down,
                    KeyCode::KeyZ => kb.z = down,
                    KeyCode::Enter => kb.enter = down,
                    KeyCode::ArrowUp => kb.up = down,
                    KeyCode::ArrowDown => kb.down = down,
                    KeyCode::ArrowLeft => kb.left = down,
                    KeyCode::ArrowRight => kb.right = down,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}

fn draw_demo(gx: &mut GxState, pads: &[crate::pad::PadStatus; 4], frames: u64, t: f32) {
    gx.copy_clear(14, 16, 24, 255, 1.0);
    gx.set_viewport(0.0, 0.0, 640.0, 480.0, 0.0, 1.0);
    gx.set_cull(GX_CULL_NONE);
    gx.set_z_mode(true, crate::gx::GX_LEQUAL, true);
    gx.set_blend(GX_BM_BLEND, 4, 5, 0);
    gx.set_tev_op(0, GX_MODULATE);

    gx.perspective(55.0, 640.0 / 480.0, 0.1, 200.0);
    let orbit = t * 0.35;
    gx.look_at(
        glam::Vec3::new(orbit.cos() * 8.0, 4.5, orbit.sin() * 8.0),
        glam::Vec3::new(0.0, 0.4, 0.0),
        glam::Vec3::Y,
    );

    gx.begin(GX_TRIANGLES, 0, 96);
    let segs = 32i32;
    for i in 0..segs {
        let a0 = i as f32 / segs as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / segs as f32 * std::f32::consts::TAU;
        let shade = 0.35 + 0.15 * ((i % 2) as f32);
        gx.color4u8(
            (40.0 + shade * 40.0) as u8,
            (70.0 + shade * 80.0) as u8,
            (160.0 + shade * 40.0) as u8,
            255,
        );
        gx.normal3f32(0.0, 1.0, 0.0);
        gx.position3f32(0.0, 0.0, 0.0);
        gx.position3f32(a0.cos() * 3.4, 0.0, a0.sin() * 3.4);
        gx.position3f32(a1.cos() * 3.4, 0.0, a1.sin() * 3.4);
    }

    let spin = t * 1.1;
    let c = spin.cos();
    let s = spin.sin();
    gx.load_pos_mtx(&[c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 1.15, s, 0.0, c, 0.0], 0);
    draw_cube(gx, 0.7);

    gx.ortho(0.0, 640.0, 480.0, 0.0, -1.0, 1.0);
    gx.load_pos_mtx(&crate::gx::mtx_identity(), 0);
    gx.set_z_mode(false, crate::gx::GX_ALWAYS, false);
    draw_hud(gx, pads, frames);
}

fn draw_cube(gx: &mut GxState, h: f32) {
    let faces: [([f32; 3], [u8; 4]); 6] = [
        ([0.0, 0.0, 1.0], [76, 110, 245, 255]),
        ([0.0, 0.0, -1.0], [240, 140, 0, 255]),
        ([1.0, 0.0, 0.0], [61, 214, 140, 255]),
        ([-1.0, 0.0, 0.0], [224, 49, 49, 255]),
        ([0.0, 1.0, 0.0], [232, 228, 217, 255]),
        ([0.0, -1.0, 0.0], [42, 46, 60, 255]),
    ];
    gx.begin(GX_QUADS, 0, 24);
    for (n, col) in faces {
        gx.normal3f32(n[0], n[1], n[2]);
        gx.color4u8(col[0], col[1], col[2], col[3]);
        let (u, v) = perp(n);
        let p = |a: f32, b: f32| {
            [
                (n[0] + u[0] * a + v[0] * b) * h,
                (n[1] + u[1] * a + v[1] * b) * h,
                (n[2] + u[2] * a + v[2] * b) * h,
            ]
        };
        for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let q = p(a, b);
            gx.position3f32(q[0], q[1], q[2]);
        }
    }
}

fn perp(n: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let up = if n[1].abs() > 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = [
        up[1] * n[2] - up[2] * n[1],
        up[2] * n[0] - up[0] * n[2],
        up[0] * n[1] - up[1] * n[0],
    ];
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    (u, v)
}

fn draw_hud(gx: &mut GxState, pads: &[crate::pad::PadStatus; 4], frames: u64) {
    let colors = [
        [224, 49, 49, 255],
        [28, 126, 214, 255],
        [245, 159, 0, 255],
        [47, 158, 68, 255],
    ];
    for (i, pad) in pads.iter().enumerate() {
        let x0 = 24.0 + i as f32 * 156.0;
        let y0 = 360.0;
        quad(
            gx,
            x0,
            y0,
            144.0,
            104.0,
            if pad.err == 0 {
                [24, 28, 40, 220]
            } else {
                [18, 20, 28, 180]
            },
        );
        let c = colors[i];
        quad(gx, x0, y0, 144.0, 4.0, c);
        let cx = x0 + 40.0;
        let cy = y0 + 56.0;
        draw_gate(gx, cx, cy, 28.0, c);
        let sx = cx + (pad.stick_x as f32) * (28.0 / 80.0);
        let sy = cy - (pad.stick_y as f32) * (28.0 / 80.0);
        quad(gx, sx - 3.0, sy - 3.0, 6.0, 6.0, [232, 228, 217, 255]);
        let cx2 = x0 + 100.0;
        let cy2 = y0 + 40.0;
        draw_gate(gx, cx2, cy2, 16.0, [180, 180, 190, 255]);
        let sx2 = cx2 + (pad.substick_x as f32) * (16.0 / 80.0);
        let sy2 = cy2 - (pad.substick_y as f32) * (16.0 / 80.0);
        quad(gx, sx2 - 2.0, sy2 - 2.0, 4.0, 4.0, [240, 140, 0, 255]);
        btn(gx, x0 + 88.0, y0 + 72.0, pad.button & PAD_BUTTON_A != 0, [61, 214, 140, 255]);
        btn(gx, x0 + 76.0, y0 + 84.0, pad.button & PAD_BUTTON_B != 0, [224, 49, 49, 255]);
        btn(gx, x0 + 100.0, y0 + 84.0, pad.button & PAD_BUTTON_X != 0, [232, 228, 217, 255]);
        btn(gx, x0 + 88.0, y0 + 96.0, pad.button & PAD_BUTTON_Y != 0, [245, 159, 0, 255]);
        btn(gx, x0 + 120.0, y0 + 72.0, pad.button & PAD_BUTTON_START != 0, [180, 180, 200, 255]);
        btn(gx, x0 + 120.0, y0 + 88.0, pad.button & PAD_TRIGGER_Z != 0, [140, 90, 220, 255]);
        let lh = pad.trigger_left as f32 / 255.0 * 40.0;
        let rh = pad.trigger_right as f32 / 255.0 * 40.0;
        quad(gx, x0 + 8.0, y0 + 96.0 - lh, 8.0, lh, [76, 110, 245, 255]);
        quad(gx, x0 + 18.0, y0 + 96.0 - rh, 8.0, rh, [240, 140, 0, 255]);
        let _ = (PAD_TRIGGER_L, PAD_TRIGGER_R, frames);
    }
}

fn quad(gx: &mut GxState, x: f32, y: f32, w: f32, h: f32, c: [u8; 4]) {
    gx.begin(GX_QUADS, 0, 4);
    gx.color4u8(c[0], c[1], c[2], c[3]);
    gx.position3f32(x, y, 0.0);
    gx.position3f32(x + w, y, 0.0);
    gx.position3f32(x + w, y + h, 0.0);
    gx.position3f32(x, y + h, 0.0);
}

fn btn(gx: &mut GxState, x: f32, y: f32, on: bool, c: [u8; 4]) {
    let col = if on { c } else { [40, 44, 56, 255] };
    quad(gx, x, y, 10.0, 10.0, col);
}

fn draw_gate(gx: &mut GxState, cx: f32, cy: f32, r: f32, c: [u8; 4]) {
    gx.begin(GX_LINES, 0, 16);
    gx.color4u8(c[0], c[1], c[2], 180);
    for i in 0..8 {
        let a0 = i as f32 / 8.0 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / 8.0 * std::f32::consts::TAU;
        gx.position3f32(cx + a0.cos() * r, cy + a0.sin() * r, 0.0);
        gx.position3f32(cx + a1.cos() * r, cy + a1.sin() * r, 0.0);
    }
}
