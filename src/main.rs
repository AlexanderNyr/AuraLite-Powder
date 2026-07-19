// temp new main
#![allow(dead_code, unused_assignments, clippy::if_same_then_else, clippy::too_many_arguments, unreachable_code)]

use aura_lite_core::{SimulationState, Particle};
use aura_lite_renderer::{color_for_element, Camera as RendererCamera};
use aura_lite_ui::{AppState, brush::BrushTool};
use aura_lite_utils::Vec2;

#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use std::sync::Arc;
#[cfg(any(feature = "softbuffer-renderer", feature = "wgpu-renderer"))]
use std::time::{Instant, Duration};

#[cfg(feature = "softbuffer-renderer")]
use pixels::{Pixels, SurfaceTexture};
#[cfg(feature = "softbuffer-renderer")]
use winit::{
    event::{Event, WindowEvent, MouseButton, ElementState, MouseScrollDelta},
    event_loop::{EventLoop, ControlFlow},
    dpi::LogicalSize,
    window::WindowBuilder,
};

#[cfg(feature = "native-ui")]
use egui::{Context as EguiContext};
#[cfg(feature = "native-ui")]
use egui_winit::State as EguiWinitState;

const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;
const GRID_WIDTH: u32 = 256;
const GRID_HEIGHT: u32 = 256;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("Starting AuraLite Powder v{} - native binary", env!("CARGO_PKG_VERSION"));

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

fn run_headless_test() {
    let mut sim = SimulationState::new(256, 256, 42);
    sim.grid.set(128, 100, Particle::new(aura_lite_core::element_id::U235, 400));
    sim.grid.set(129, 100, Particle::new(aura_lite_core::element_id::U235, 400));
    sim.grid.set(130, 100, Particle::new(aura_lite_core::element_id::U235, 400));
    sim.grid.set(128, 101, Particle::new(aura_lite_core::element_id::NEUTRON_THERMAL, 350));
    println!("Initial particles: {}", sim.grid.count_non_empty());
    for i in 0..100 {
        sim.tick();
        if i % 10 == 0 {
            println!("Tick {}: particles {}, fission {}, fusion {}, reactions {}", i, sim.grid.count_non_empty(), sim.fission_count, sim.fusion_count, sim.reaction_count);
        }
    }
    println!("Final: fission={}, fusion={}, reactions={}", sim.fission_count, sim.fusion_count, sim.reaction_count);
}

#[cfg(feature = "softbuffer-renderer")]
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
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window.clone());
    let mut pixels = Pixels::new(GRID_WIDTH, GRID_HEIGHT, surface_texture)?;

    let mut app_state = AppState::new(GRID_WIDTH, GRID_HEIGHT);
    demo_setup(&mut app_state.simulation);

    let mut camera = RendererCamera::new(window_size.width as f32, window_size.height as f32);
    let mut mouse_pos = Vec2::new(0.0, 0.0);
    let mut mouse_down = false;
    let mut right_mouse_down = false;
    let mut last_mouse_pos = Vec2::new(0.0, 0.0);
    let mut line_start: Option<(i32,i32)> = None;

    let mut last_tick = Instant::now();
    let mut fps_accum = 0u32;
    let mut fps_counter = 0.0;
    let mut fps_instant = Instant::now();
    let mut tick_accumulator = Duration::ZERO;

    #[cfg(feature = "native-ui")]
    let egui_ctx = EguiContext::default();
    #[cfg(feature = "native-ui")]
    let mut egui_state = EguiWinitState::new(egui_ctx.clone(), egui::ViewportId::ROOT, window.as_ref(), None, None);
    #[cfg(feature = "native-ui")]
    let show_egui = true;

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
                let _ = pixels.resize_surface(size.width, size.height);
                let _ = pixels.resize_buffer(GRID_WIDTH, GRID_HEIGHT);
                camera.resize(size.width as f32, size.height as f32);
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
                if mouse_down {
                    let world = camera.screen_to_world(mouse_pos);
                    let gx = world.x as i32;
                    let gy = world.y as i32;
                    apply_brush(&mut app_state, gx, gy, false);
                }
            }
            Event::WindowEvent { event: WindowEvent::MouseInput { state, button, .. }, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
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
                            if let Err(e) = aura_lite_io::save_to_file(&path, &app_state.simulation.grid, &app_state.simulation.settings, false) {
                                log::error!("Quick save failed: {}", e);
                            } else {
                                log::info!("Quick saved to {:?}", path);
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
                        "AuraLite Powder | FPS: {:.1} | Particles: {} | Fission: {} Fusion: {} | Selected: {} | [1]Sand [2]Water [3]U235 [4]Neutron [5]D [6]T",
                        fps_counter,
                        app_state.info.particle_count,
                        app_state.simulation.fission_count,
                        app_state.simulation.fusion_count,
                        aura_lite_elements::registry::name_for_id(app_state.palette.selected_id)
                    );
                    window_clone.set_title(&title);
                }
                window_clone.request_redraw();
            }
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                #[cfg(feature = "native-ui")]
                {
                    if show_egui {
                        let raw_input = egui_state.take_egui_input(window_clone.as_ref());
                        let full_output = egui_ctx.run(raw_input, |ctx| {
                            aura_lite_ui::egui_ui::build_ui(ctx, &mut app_state);
                        });
                        egui_state.handle_platform_output(window_clone.as_ref(), full_output.platform_output);
                    }
                }
                let frame = pixels.frame_mut();
                let sz = window_clone.inner_size();
                render_simulation_to_frame(&app_state.simulation, frame, &camera, (sz.width, sz.height));
                apply_temperature_overlay(&app_state.simulation, frame);
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

fn apply_brush(app: &mut AppState, gx: i32, gy: i32, is_start: bool) {
    match app.tools.brush.tool {
        BrushTool::Brush | BrushTool::Eraser => {
            app.tools.brush.apply_brush(&mut app.simulation.grid, gx, gy);
        }
        BrushTool::Line => {
            if !is_start {
                app.tools.brush.apply_brush(&mut app.simulation.grid, gx, gy);
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

fn render_simulation_to_frame(sim: &SimulationState, frame: &mut [u8], camera: &RendererCamera, window_size: (u32,u32)) {
    let win_w = window_size.0 as usize;
    let win_h = window_size.1 as usize;
    let grid_w = sim.grid.width as usize;
    let grid_h = sim.grid.height as usize;

    let scale = camera.scale;
    let offset_x = camera.offset.x;
    let offset_y = camera.offset.y;

    if (scale - 1.0).abs() < 0.01 && offset_x.abs() < 0.5 && offset_y.abs() < 0.5 {
        if win_w == grid_w && win_h == grid_h && frame.len() == grid_w*grid_h*4 {
            for (i, p) in sim.grid.particles.iter().enumerate() {
                let col = color_for_element(p.element_id);
                let base = i*4;
                if base+3 < frame.len() {
                    frame[base] = col[0];
                    frame[base+1] = col[1];
                    frame[base+2] = col[2];
                    frame[base+3] = 255;
                    if p.is_empty() {
                        frame[base] = 10;
                        frame[base+1] = 10;
                        frame[base+2] = 15;
                        frame[base+3] = 255;
                    }
                }
            }
        } else {
            for y in 0..win_h {
                for x in 0..win_w {
                    let gx = (x as f32 * grid_w as f32 / win_w as f32) as usize;
                    let gy = (y as f32 * grid_h as f32 / win_h as f32) as usize;
                    let grid_idx = gy.min(grid_h-1) * grid_w + gx.min(grid_w-1);
                    let p = sim.grid.particles[grid_idx];
                    let col = color_for_element(p.element_id);
                    let frame_idx = (y * win_w + x)*4;
                    if frame_idx+3 < frame.len() {
                        if p.is_empty() {
                            frame[frame_idx] = 10;
                            frame[frame_idx+1] = 10;
                            frame[frame_idx+2] = 15;
                            frame[frame_idx+3] = 255;
                        } else {
                            frame[frame_idx] = col[0];
                            frame[frame_idx+1] = col[1];
                            frame[frame_idx+2] = col[2];
                            frame[frame_idx+3] = 255;
                        }
                    }
                }
            }
        }
    } else {
        for y in 0..win_h {
            for x in 0..win_w {
                let screen = Vec2::new(x as f32, y as f32);
                let world = camera.screen_to_world(screen);
                let gx = world.x as i32;
                let gy = world.y as i32;
                let frame_idx = (y * win_w + x)*4;
                if frame_idx+3 >= frame.len() { continue; }
                if gx >=0 && gy >=0 && (gx as usize) < grid_w && (gy as usize) < grid_h {
                    let p = sim.grid.particles[gy as usize * grid_w + gx as usize];
                    if p.is_empty() {
                        frame[frame_idx]=10; frame[frame_idx+1]=10; frame[frame_idx+2]=15; frame[frame_idx+3]=255;
                    } else {
                        let col = color_for_element(p.element_id);
                        frame[frame_idx]=col[0]; frame[frame_idx+1]=col[1]; frame[frame_idx+2]=col[2]; frame[frame_idx+3]=255;
                    }
                } else {
                    frame[frame_idx]=5; frame[frame_idx+1]=5; frame[frame_idx+2]=10; frame[frame_idx+3]=255;
                }
            }
        }
    }
}

fn apply_temperature_overlay(sim: &SimulationState, frame: &mut [u8]) {
    if frame.len() != (sim.grid.width * sim.grid.height * 4) as usize {
        return;
    }
    for (i,p) in sim.grid.particles.iter().enumerate() {
        if p.temperature > 800 && !p.is_empty() {
            let base = i*4;
            let factor = ((p.temperature as f32 - 800.0)/2000.0).clamp(0.0,1.0);
            if base+2 < frame.len() {
                frame[base]=frame[base].saturating_add((factor*200.0) as u8);
                frame[base+1]=frame[base+1].saturating_add((factor*100.0) as u8);
            }
        }
    }
}

fn demo_setup(sim: &mut SimulationState) {
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        sim.grid.set(x, h-2, Particle::new(aura_lite_core::element_id::CONCRETE, 293));
        sim.grid.set(x, h-1, Particle::new(aura_lite_core::element_id::CONCRETE, 293));
    }
    for y in h-20..h {
        sim.grid.set(0, y, Particle::new(aura_lite_core::element_id::CONCRETE, 293));
        sim.grid.set(w-1, y, Particle::new(aura_lite_core::element_id::CONCRETE, 293));
    }
    for y in h-12..h-5 {
        for x in w/2-8..w/2+8 {
            if fastrand::bool() {
                sim.grid.set(x, y, Particle::new(aura_lite_core::element_id::U235, 350));
            }
        }
    }
    for y in h-15..h-12 {
        for x in w/2-10..w/2+10 {
            sim.grid.set(x, y, Particle::new(aura_lite_core::element_id::GRAPHITE, 300));
        }
    }
    sim.grid.set(w/2, h-14, Particle::new(aura_lite_core::element_id::NEUTRON_THERMAL, 350));
    for y in h-15..h-2 {
        sim.grid.set(w/2-12, y, Particle::new(aura_lite_core::element_id::BORON, 293));
        sim.grid.set(w/2+12, y, Particle::new(aura_lite_core::element_id::BORON, 293));
    }
    sim.grid.set(30, 30, Particle::new(aura_lite_core::element_id::DEUTERIUM, 1600));
    sim.grid.set(31, 30, Particle::new(aura_lite_core::element_id::TRITIUM, 1600));
}

#[cfg(feature = "wgpu-renderer")]
fn run_with_wgpu() -> anyhow::Result<()> {
    println!("WGPU renderer selected - validating shader");
    let shader_code = std::fs::read_to_string("assets/shaders/shader.wgsl")
        .unwrap_or_else(|_| aura_lite_renderer::wgpu_backend::WgpuBackend::load_shader());
    println!("Loaded shader {} bytes", shader_code.len());
    #[cfg(feature = "softbuffer-renderer")]
    {
        run_with_softbuffer()
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
