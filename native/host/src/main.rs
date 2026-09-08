//! `melee-native` — Linux executable.

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let headless = std::env::args().any(|a| a == "--headless") || !cfg!(feature = "window");
    if headless {
        run_headless();
        return;
    }
    #[cfg(feature = "window")]
    {
        use melee_host::HostApp;
        use winit::event_loop::{ControlFlow, EventLoop};
        let event_loop = EventLoop::new().expect("event loop");
        event_loop.set_control_flow(ControlFlow::Poll);
        let mut app = HostApp::new(false);
        event_loop.run_app(&mut app).expect("run");
    }
}

fn run_headless() {
    melee_host::os::init();
    melee_host::dvd::init();
    let mut pad = melee_host::PadSystem::new();
    let mut gx = melee_host::GxState::new();
    for i in 0..60 {
        let _ = pad.poll();
        gx.abort_frame();
        gx.copy_clear(14, 16, 24, 255, 1.0);
        gx.begin(melee_host::gx::GX_TRIANGLES, 0, 3);
        gx.color4u8(76, 110, 245, 255);
        gx.position3f32(-0.5, -0.5, 0.0);
        gx.position3f32(0.5, -0.5, 0.0);
        gx.position3f32(0.0, 0.5, 0.0);
        let cmds = gx.take_cmds();
        assert_eq!(cmds.len(), 1);
        if i == 0 {
            eprintln!(
                "melee-native headless: pad kinds {:?}",
                pad.kinds().map(|k| k.label())
            );
        }
    }
    eprintln!("melee-native headless: 60 frames of GX + PAD ok");
}
