#![cfg_attr(
    not(any(feature = "softbuffer-renderer", feature = "wgpu-renderer")),
    allow(dead_code, unreachable_code)
)]

use aura_lite_core::SimulationState;
#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use aura_lite_renderer::{render_simulation, Camera as RendererCamera};
#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use aura_lite_ui::{brush::BrushTool, AppState};
#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use aura_lite_utils::Vec2;

#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use std::sync::Arc;
#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use std::time::{Duration, Instant};

#[cfg(feature = "softbuffer-renderer")]
use pixels::{Pixels, SurfaceTexture};
#[cfg(feature = "softbuffer-renderer")]
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(feature = "native-ui")]
use egui::Context as EguiContext;
#[cfg(feature = "native-ui")]
use egui_winit::State as EguiWinitState;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;
const GRID_WIDTH: u32 = 256;
const GRID_HEIGHT: u32 = 256;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!(
        "Starting AuraLite Powder v{} - native binary",
        env!("CARGO_PKG_VERSION")
    );

    #[cfg(not(any(feature = "softbuffer-renderer", feature = "wgpu-renderer")))]
    {
        println!("No renderer feature enabled, running headless simulation benchmark...");
        run_headless_test();
        return Ok(());
    }

    #[cfg(feature = "softbuffer-renderer")]
    {
        run_with_softbuffer()?;
    }

    #[cfg(all(feature = "wgpu-renderer", not(feature = "softbuffer-renderer")))]
    {
        run_with_wgpu()?;
    }

    Ok(())
}

#[allow(dead_code)]
fn run_headless_test() {
    let mut sim = SimulationState::new(256, 256, 42);
    sim.grid.set(
        128,
        100,
        aura_lite_core::Particle::new(aura_lite_core::element_id::U235, 400),
    );
    sim.grid.set(
        129,
        100,
        aura_lite_core::Particle::new(aura_lite_core::element_id::U235, 400),
    );
    sim.grid.set(
        130,
        100,
        aura_lite_core::Particle::new(aura_lite_core::element_id::U235, 400),
    );
    sim.grid.set(
        128,
        101,
        aura_lite_core::Particle::new(aura_lite_core::element_id::NEUTRON_THERMAL, 350),
    );
    println!("Initial particles: {}", sim.grid.count_non_empty());
    for i in 0..100 {
        sim.tick();
        if i % 10 == 0 {
            println!(
                "Tick {}: particles {}, fission {}, fusion {}, reactions {}",
                i,
                sim.grid.count_non_empty(),
                sim.fission_count,
                sim.fusion_count,
                sim.reaction_count
            );
        }
    }
    println!(
        "Final: fission={}, fusion={}, reactions={}",
        sim.fission_count, sim.fusion_count, sim.reaction_count
    );
}

#[cfg(feature = "softbuffer-renderer")]
#[allow(unused_assignments)]
fn run_with_softbuffer() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("AuraLite Powder - Nuclear Falling SandSim")
            .with_inner_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
            .with_min_inner_size(LogicalSize::new(400, 300))
            .build(&event_loop)?,
    );

    let window_size = window.inner_size();
    let surface_texture =
        SurfaceTexture::new(window_size.width, window_size.height, window.clone());
    // Framebuffer matches the window so egui and the camera share the same space.
    let mut pixels = Pixels::new(window_size.width, window_size.height, surface_texture)?;

    let mut app_state = AppState::new(GRID_WIDTH, GRID_HEIGHT);
    app_state.simulation.setup_reactor_demo();

    let mut camera = RendererCamera::new(window_size.width as f32, window_size.height as f32);
    camera.scale = (window_size.width as f32 / GRID_WIDTH as f32)
        .min(window_size.height as f32 / GRID_HEIGHT as f32)
        .max(1.0);
    let mut mouse_pos = Vec2::new(0.0, 0.0);
    let mut mouse_down = false;
    let mut right_mouse_down = false;
    let mut last_mouse_pos = Vec2::new(0.0, 0.0);
    let mut line_start: Option<(i32, i32)> = None;

    let mut last_tick = Instant::now();
    let mut fps_accum = 0u32;
    let mut fps_counter = 0.0_f32;
    let mut fps_instant = Instant::now();
    let mut tick_accumulator = Duration::ZERO;

    #[cfg(feature = "native-ui")]
    let egui_ctx = EguiContext::default();
    #[cfg(feature = "native-ui")]
    let mut egui_state = EguiWinitState::new(
        egui_ctx.clone(),
        egui::ViewportId::ROOT,
        window.as_ref(),
        None,
        None,
    );
    #[cfg(feature = "native-ui")]
    let show_egui = true;
    #[cfg(feature = "native-ui")]
    let mut egui_textures = aura_lite_ui::egui_raster::EguiTextures::new();

    // We need to move window Arc clones into closure, but also keep window for title set inside closure via Arc
    let window_clone = window.clone();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        if let Event::WindowEvent { event: ref win_event, .. } = event {
            #[cfg(feature = "native-ui")]
            {
                let _ = egui_state.on_window_event(window_clone.as_ref(), win_event);
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                elwt.exit();
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                if size.width > 0 && size.height > 0 {
                    let _ = pixels.resize_surface(size.width, size.height);
                    let _ = pixels.resize_buffer(size.width, size.height);
                    camera.resize(size.width as f32, size.height as f32);
                }
            }
            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                mouse_pos = Vec2::new(position.x as f32, position.y as f32);
                let world = camera.screen_to_world(mouse_pos);
                if world.x >= 0.0 && world.y >= 0.0 {
                    app_state.inspector.hovered_x = Some(world.x as u32);
                    app_state.inspector.hovered_y = Some(world.y as u32);
                }
                if right_mouse_down {
                    let delta = Vec2::new(mouse_pos.x - last_mouse_pos.x, mouse_pos.y - last_mouse_pos.y);
                    camera.pan(delta);
                }
                last_mouse_pos = mouse_pos;
                let egui_busy = {
                    #[cfg(feature = "native-ui")]
                    {
                        egui_ctx.wants_pointer_input()
                    }
                    #[cfg(not(feature = "native-ui"))]
                    {
                        false
                    }
                };
                if mouse_down && !egui_busy {
                    let world = camera.screen_to_world(mouse_pos);
                    let gx = world.x as i32;
                    let gy = world.y as i32;
                    apply_brush(&mut app_state, gx, gy, false);
                }
            }
            Event::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        let egui_busy = {
                            #[cfg(feature = "native-ui")]
                            {
                                egui_ctx.wants_pointer_input()
                            }
                            #[cfg(not(feature = "native-ui"))]
                            {
                                false
                            }
                        };
                        if !egui_busy {
                            mouse_down = true;
                            let world = camera.screen_to_world(mouse_pos);
                            let gx = world.x as i32;
                            let gy = world.y as i32;
                            if matches!(app_state.tools.brush.tool, BrushTool::Line | BrushTool::Rectangle) {
                                line_start = Some((gx, gy));
                            } else {
                                apply_brush(&mut app_state, gx, gy, true);
                            }
                        }
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        mouse_down = false;
                        if let Some(start) = line_start.take() {
                            let world = camera.screen_to_world(mouse_pos);
                            let gx = world.x as i32;
                            let gy = world.y as i32;
                            match app_state.tools.brush.tool {
                                BrushTool::Line => {
                                    app_state.tools.brush.apply_line(&mut app_state.simulation.grid, start.0, start.1, gx, gy);
                                }
                                BrushTool::Rectangle => {
                                    app_state.tools.brush.apply_rectangle(&mut app_state.simulation.grid, start.0, start.1, gx, gy, false);
                                }
                                _ => {}
                            }
                        }
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        right_mouse_down = true;
                        last_mouse_pos = mouse_pos;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        right_mouse_down = false;
                    }
                    _ => {}
                }
            }
            Event::WindowEvent { event: WindowEvent::MouseWheel { delta, .. }, .. } => {
                let factor = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if y > 0.0 { 1.1 } else { 0.9 }
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        if pos.y > 0.0 { 1.05 } else { 0.95 }
                    }
                };
                camera.zoom(factor, Some(mouse_pos));
            }
            Event::WindowEvent { event: WindowEvent::KeyboardInput { event, .. }, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    match event.logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                            app_state.controller.paused = !app_state.controller.paused;
                        }
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                            elwt.exit();
                        }
                        winit::keyboard::Key::Character(c) if c == "c" || c == "C" => {
                            app_state.simulation.grid.clear();
                        }
                        winit::keyboard::Key::Character(c) if c == "s" || c == "S" => {
                            let path = std::env::temp_dir().join("auralite_quick.aura");
                            if let Err(e) = aura_lite_io::save_simulation_to_file(&path, &app_state.simulation, false) {
                                log::error!("Quick save failed: {}", e);
                            } else {
                                log::info!("Quick saved to {:?}", path);
                            }
                        }
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::F12) => {
                            let sz = window_clone.inner_size();
                            let frame = pixels.frame();
                            if let Err(e) = image::save_buffer(
                                "auralite_screenshot.png",
                                frame,
                                sz.width,
                                sz.height,
                                image::ColorType::Rgba8,
                            ) {
                                log::error!("Screenshot failed: {}", e);
                            } else {
                                log::info!("Wrote auralite_screenshot.png");
                            }
                        }
                        winit::keyboard::Key::Character(c) if c == "1" => { app_state.set_selected_element(1); }
                        winit::keyboard::Key::Character(c) if c == "2" => { app_state.set_selected_element(2); }
                        winit::keyboard::Key::Character(c) if c == "3" => { app_state.set_selected_element(4); }
                        winit::keyboard::Key::Character(c) if c == "4" => { app_state.set_selected_element(13); }
                        winit::keyboard::Key::Character(c) if c == "5" => { app_state.set_selected_element(21); }
                        winit::keyboard::Key::Character(c) if c == "6" => { app_state.set_selected_element(20); }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                let now = Instant::now();
                let dt = now.duration_since(last_tick);
                last_tick = now;
                tick_accumulator += dt.mul_f64(app_state.controller.speed as f64);
                let target_tick = Duration::from_secs_f64(1.0 / app_state.controller.tick_rate as f64);
                while tick_accumulator >= target_tick {
                    if !app_state.controller.paused {
                        app_state.simulation.tick();
                    }
                    tick_accumulator -= target_tick;
                }
                fps_accum += 1;
                let elapsed = fps_instant.elapsed();
                if elapsed >= Duration::from_secs(1) {
                    fps_counter = fps_accum as f32 / elapsed.as_secs_f32();
                    app_state.update_info(fps_counter);
                    fps_accum = 0;
                    fps_instant = Instant::now();
                    let title = format!(
                        "AuraLite Powder | FPS: {:.1} | Particles: {} | Fission: {} Fusion: {} | k-eff: {:.2} | Selected: {} | [1]Sand [2]Water [3]U235 [4]Neutron [5]D [6]T",
                        fps_counter,
                        app_state.info.particle_count,
                        app_state.simulation.fission_count,
                        app_state.simulation.fusion_count,
                        app_state.simulation.k_effective,
                        aura_lite_elements::registry::name_for_id(app_state.palette.selected_id)
                    );
                    window_clone.set_title(&title);
                }
                window_clone.request_redraw();
            }
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                let sz = window_clone.inner_size();
                {
                    let frame = pixels.frame_mut();
                    render_simulation(
                        &app_state.simulation,
                        frame,
                        sz.width,
                        sz.height,
                        &camera,
                    );
                }
                #[cfg(feature = "native-ui")]
                {
                    if show_egui {
                        let raw_input = egui_state.take_egui_input(window_clone.as_ref());
                        let full_output = egui_ctx.run(raw_input, |ctx| {
                            aura_lite_ui::egui_ui::build_ui(ctx, &mut app_state);
                        });
                        egui_textures.apply(&full_output.textures_delta);
                        let primitives = egui_ctx.tessellate(
                            full_output.shapes,
                            full_output.pixels_per_point,
                        );
                        aura_lite_ui::egui_raster::rasterize(
                            pixels.frame_mut(),
                            sz.width,
                            sz.height,
                            full_output.pixels_per_point,
                            &egui_textures,
                            &primitives,
                        );
                        egui_state.handle_platform_output(
                            window_clone.as_ref(),
                            full_output.platform_output,
                        );
                    }
                }
                if let Err(e) = pixels.render() {
                    log::error!("pixels render error: {}", e);
                    elwt.exit();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
fn apply_brush(app: &mut AppState, gx: i32, gy: i32, is_start: bool) {
    match app.tools.brush.tool {
        BrushTool::Brush | BrushTool::Eraser => {
            app.tools
                .brush
                .apply_brush(&mut app.simulation.grid, gx, gy);
        }
        BrushTool::Line => {
            if !is_start {
                app.tools
                    .brush
                    .apply_brush(&mut app.simulation.grid, gx, gy);
            }
        }
        BrushTool::Fill => {
            if is_start {
                app.tools.brush.apply_fill(&mut app.simulation.grid, gx, gy);
            }
        }
        BrushTool::Rectangle => {}
    }
}

#[cfg(feature = "wgpu-renderer")]
#[allow(dead_code)]
fn run_with_wgpu() -> anyhow::Result<()> {
    use aura_lite_renderer::backend::RenderBackend;
    use aura_lite_renderer::wgpu_backend::WgpuBackend;
    println!("WGPU renderer selected - creating device and validating shader");
    let mut backend = WgpuBackend::init(GRID_WIDTH, GRID_HEIGHT);
    let shader_code = std::fs::read_to_string("assets/shaders/shader.wgsl")
        .unwrap_or_else(|_| WgpuBackend::load_shader());
    println!(
        "Loaded shader {} bytes (validated={})",
        shader_code.len(),
        backend.shader_validated()
    );
    let mut sim = SimulationState::new(GRID_WIDTH, GRID_HEIGHT, 42);
    sim.setup_reactor_demo();
    let pixels = aura_lite_renderer::render_grid_with_glow(&sim);
    backend.render(&pixels);
    #[cfg(feature = "softbuffer-renderer")]
    {
        return run_with_softbuffer();
    }
    #[cfg(not(feature = "softbuffer-renderer"))]
    {
        run_headless_test();
        Ok(())
    }
}

mod anyhow {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
