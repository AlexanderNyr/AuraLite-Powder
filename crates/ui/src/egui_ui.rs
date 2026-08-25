//! Egui integration for native UI

#[cfg(feature = "native-ui")]
use egui::{Context, Ui};

#[cfg(feature = "native-ui")]
use crate::components::app_state::AppState;

#[cfg(feature = "native-ui")]
pub fn show_palette(ui: &mut Ui, app: &mut AppState) {
    ui.heading("Element Palette");
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut app.palette.search_query);
    });
    ui.label("Hotbar 1–0  (comma/period cycle)");
    ui.horizontal_wrapped(|ui| {
        let hotbar = app.palette.hotbar;
        for (i, &id) in hotbar.iter().enumerate() {
            let name = aura_lite_elements::registry::name_for_id(id);
            let key = if i == 9 { 0 } else { i + 1 };
            let label = format!("{key}:{name}");
            let selected = app.palette.selected_id == id;
            let btn = if selected {
                egui::Button::new(label).fill(egui::Color32::from_rgb(80, 80, 200))
            } else {
                egui::Button::new(label)
            };
            if ui.add(btn).clicked() {
                app.set_selected_element(id);
            }
        }
    });
    if !app.palette.favorites.is_empty() {
        ui.label("Favorites");
        ui.horizontal_wrapped(|ui| {
            for &id in &app.palette.favorites.clone() {
                let name = aura_lite_elements::registry::name_for_id(id);
                if ui.button(name).clicked() {
                    app.set_selected_element(id);
                }
            }
        });
    }
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for id in app.palette.elements_filtered() {
            let name = aura_lite_elements::registry::name_for_id(id);
            let color = aura_lite_elements::registry::color_for_id(id);
            let is_selected = app.palette.selected_id == id;
            let fav = app.palette.favorites.contains(&id);
            ui.horizontal(|ui| {
                let star = if fav { "★" } else { "☆" };
                if ui.small_button(star).clicked() {
                    app.toggle_favorite(id);
                }
                let btn_text = format!("{} ({})", name, id);
                let mut btn = egui::Button::new(btn_text);
                if is_selected {
                    btn = btn.fill(egui::Color32::from_rgb(80, 80, 200));
                }
                if ui.add(btn).clicked() {
                    app.set_selected_element(id);
                }
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    2.0,
                    egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                );
            });
        }
    });
}

#[cfg(feature = "native-ui")]
pub fn show_tool_panel(ui: &mut Ui, app: &mut AppState) {
    ui.heading("Tools");
    ui.horizontal(|ui| {
        ui.label("Brush radius:");
        ui.add(egui::Slider::new(&mut app.tools.brush.radius, 1..=20));
    });
    ui.horizontal(|ui| {
        if ui.button("Brush").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Brush;
        }
        if ui.button("Line").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Line;
        }
        if ui.button("Fill").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Fill;
        }
        if ui.button("Eraser").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Eraser;
        }
        if ui.button("Rect").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Rectangle;
        }
        if ui.button("Copy").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Copy;
        }
        if ui.button("Stamp").clicked() {
            app.tools.brush.tool = crate::brush::BrushTool::Stamp;
        }
    });
    ui.label(format!("Current tool: {:?}", app.tools.brush.tool));
    ui.add(egui::Slider::new(&mut app.tools.brush.temperature, 0..=5000).text("Temperature"));
    ui.label(format!("Clipboard: {} cells", app.clipboard.len()));
}

#[cfg(feature = "native-ui")]
pub fn show_property_inspector(ui: &mut Ui, app: &AppState) {
    ui.heading("Property Inspector");
    if let Some(info) = app.inspector.inspect(&app.simulation) {
        ui.label(info);
    } else {
        ui.label("Hover over a particle to inspect");
    }
}

#[cfg(feature = "native-ui")]
pub fn show_info_panel(ui: &mut Ui, app: &AppState) {
    ui.heading("Info");
    ui.label(format!("FPS: {:.1}", app.info.fps));
    ui.label(format!("Tick rate: {:.1}", app.info.tick_rate));
    ui.label(format!("Particles: {}", app.info.particle_count));
    ui.label(format!("Reactions: {}", app.info.active_reactions));
    ui.label(format!("Fission: {}", app.simulation.fission_count));
    ui.label(format!("Fusion: {}", app.simulation.fusion_count));
    ui.label(format!("Decay: {}", app.simulation.decay_count));
    ui.label(format!("k-eff: {:.3}", app.simulation.k_effective));
    ui.label(format!(
        "k-measured: {:.3}  (from the fission chain; 1.0 = self-sustaining)",
        app.simulation.k_measured
    ));
    ui.label(format!(
        "Power: {:.2}  {}",
        app.simulation.power,
        app.simulation.reactor_status()
    ));
    if app.simulation.period_ticks.abs() > 0.5 {
        ui.label(format!("Period: {:+.0} ticks", app.simulation.period_ticks));
    }
    ui.label("Poison (I → Xe)");
    let poison = (app.simulation.iodine_count as f32 * 0.4 + app.simulation.xenon_count as f32)
        .min(80.0)
        / 80.0;
    ui.add(egui::ProgressBar::new(poison).text(format!(
        "I {}  Xe {}",
        app.simulation.iodine_count, app.simulation.xenon_count
    )));
    ui.label(format!(
        "Neutron queue: {}",
        app.simulation.neutron_queue.len()
    ));
}

#[cfg(feature = "native-ui")]
pub fn show_simulation_controller(ui: &mut Ui, app: &mut AppState) {
    ui.heading("Simulation");
    ui.horizontal(|ui| {
        if ui
            .button(if app.controller.paused {
                "Resume"
            } else {
                "Pause"
            })
            .clicked()
        {
            app.controller.paused = !app.controller.paused;
        }
        if ui.button("Step").clicked() {
            app.simulation.tick();
        }
        if ui.button("Clear").clicked() {
            app.push_undo();
            app.simulation.grid.clear();
        }
        if ui.button("Undo").clicked() {
            app.undo();
        }
    });
    ui.add(egui::Slider::new(&mut app.controller.speed, 0.1..=5.0).text("Speed"));
    ui.add(egui::Slider::new(&mut app.controller.tick_rate, 1..=120).text("Tick rate"));
    ui.horizontal(|ui| {
        ui.label("Overlay:");
        if ui.button(app.overlay.label()).clicked() {
            app.overlay = app.overlay.next();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Rods up").clicked() {
            app.simulation.shift_control_rods(-2);
        }
        if ui.button("Rods down").clicked() {
            app.simulation.shift_control_rods(2);
        }
        if ui.button("Help").clicked() {
            app.show_tutorial = !app.show_tutorial;
        }
        let rec = if app.recording { "Stop rec" } else { "Rec GIF" };
        if ui.button(rec).clicked() {
            app.recording = !app.recording;
            if app.recording {
                app.rec_frames.clear();
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Grid:");
        if ui.button("256").clicked() {
            app.resize_grid(256, 256);
        }
        if ui.button("512").clicked() {
            app.resize_grid(512, 512);
        }
        ui.label(format!(
            "{}×{}",
            app.simulation.grid.width, app.simulation.grid.height
        ));
    });
    ui.separator();
    ui.label("Scenes");
    ui.horizontal_wrapped(|ui| {
        for &scene in aura_lite_core::Scenario::all() {
            if ui.button(scene.name()).clicked() {
                app.apply_scenario(scene);
            }
        }
    });
    ui.separator();
    ui.label("Missions");
    ui.horizontal_wrapped(|ui| {
        for &id in aura_lite_core::MissionId::all() {
            if ui.button(id.title()).clicked() {
                app.start_mission(id);
            }
        }
    });
    if let Some(m) = &app.mission {
        let color = match m.status {
            aura_lite_core::MissionStatus::Won => egui::Color32::from_rgb(80, 200, 80),
            aura_lite_core::MissionStatus::Failed => egui::Color32::from_rgb(220, 70, 70),
            aura_lite_core::MissionStatus::Running => egui::Color32::from_rgb(220, 200, 80),
        };
        ui.colored_label(color, &m.message);
    }
    if app.mission.is_some() {
        ui.horizontal(|ui| {
            if ui.button("Retry").clicked() {
                app.retry_mission();
            }
            if ui.button("Continue").clicked() {
                app.controller.paused = false;
            }
            if ui.button("Abandon").clicked() {
                app.abandon_mission();
            }
        });
    }
}

#[cfg(feature = "native-ui")]
pub fn show_save_load(ui: &mut Ui, app: &mut AppState) {
    ui.heading(format!("Save/Load {}", app.save_load.version_label));
    ui.checkbox(&mut app.save_load.compression_enabled, "Compression (zstd)");
    if ui.button("Save (dialog)").clicked() {
        #[cfg(feature = "native-ui")]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Aura save", &["aura", "json"])
                .save_file()
            {
                let _ = aura_lite_io::save_simulation_to_file(
                    &path,
                    &app.simulation,
                    app.save_load.compression_enabled,
                );
                app.save_load.last_save_path = Some(path.display().to_string());
            }
        }
    }
    if ui.button("Load (dialog)").clicked() {
        #[cfg(feature = "native-ui")]
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Aura save", &["aura", "json"])
                .pick_file()
            {
                if let Ok(save) = aura_lite_io::load_save_from_file(&path) {
                    if save.apply_to(&mut app.simulation).is_ok() {
                        app.sync_mission_from_save();
                        app.save_load.last_save_path = Some(path.display().to_string());
                    }
                }
            }
        }
    }
    if let Some(p) = &app.save_load.last_save_path {
        ui.label(format!("Last: {}", p));
    }
}

#[cfg(feature = "native-ui")]
pub fn build_ui(ctx: &Context, app: &mut AppState) {
    egui::SidePanel::left("left_panel").show(ctx, |ui| {
        show_palette(ui, app);
        ui.separator();
        show_tool_panel(ui, app);
    });
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        show_simulation_controller(ui, app);
        ui.separator();
        show_property_inspector(ui, app);
        ui.separator();
        show_info_panel(ui, app);
        ui.separator();
        show_save_load(ui, app);
    });
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.heading("AuraLite Powder - Nuclear Falling Sand");
        if let Some(m) = &app.mission {
            let color = match m.status {
                aura_lite_core::MissionStatus::Won => egui::Color32::from_rgb(80, 200, 80),
                aura_lite_core::MissionStatus::Failed => egui::Color32::from_rgb(220, 70, 70),
                aura_lite_core::MissionStatus::Running => egui::Color32::from_rgb(240, 220, 90),
            };
            ui.colored_label(color, format!("{} — {}", m.id.title(), m.id.brief()));
            ui.colored_label(color, &m.message);
        }
    });
    if app.show_tutorial {
        let step = app.tutorial_step;
        let mut next = false;
        let mut skip = false;
        egui::Window::new("How to play")
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(format!("Step {} / 5", step + 1));
                ui.separator();
                match step {
                    0 => {
                        ui.label("Paint sand with hotbar key 1. Scroll-zoom, right-drag to pan.");
                    }
                    1 => {
                        ui.label("Space pauses. C clears, Z undoes.");
                    }
                    2 => {
                        ui.label("Load Bare reactor. Watch k-eff and the poison bar.");
                    }
                    3 => {
                        ui.label("[ ] move rods. Try the Hold critical mission.");
                    }
                    4 => {
                        ui.label("H overlay. Copy/Stamp. R records GIF. - / = brush size.");
                    }
                    _ => {
                        ui.label("Missions and 256/512 grid are on the right.");
                    }
                }
                ui.separator();
                if ui.button("Next").clicked() {
                    next = true;
                }
                if ui.button("Skip").clicked() {
                    skip = true;
                }
            });
        if next {
            if app.tutorial_step < 5 {
                app.tutorial_step += 1;
            } else {
                app.show_tutorial = false;
            }
        }
        if skip {
            app.show_tutorial = false;
        }
    }
}
