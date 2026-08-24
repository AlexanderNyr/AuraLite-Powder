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
        for (i, &id) in app.palette.hotbar.iter().enumerate() {
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
        "Power: {:.2}  {}",
        app.simulation.power,
        app.simulation.reactor_status()
    ));
    if app.simulation.period_ticks.abs() > 0.5 {
        ui.label(format!(
            "Period: {:+.0} ticks",
            app.simulation.period_ticks
        ));
    }
    ui.label(format!(
        "Iodine: {}   Xenon: {}",
        app.simulation.iodine_count, app.simulation.xenon_count
    ));
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
    });
    if app.show_tutorial {
        egui::Window::new("How to play")
            .open(&mut app.show_tutorial)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label("Left click: paint   Right drag: pan   Wheel: zoom");
                ui.label("Space pause · C clear · Z undo · H overlay · [ ] rods");
                ui.label("S quick-save · F12 screenshot · 1–0 hotbar · , . cycle · R record GIF");
                ui.separator();
                ui.label("Copy a rectangle (yellow preview), then Stamp it (ghost preview).");
                ui.label("Control rods soak neutrons; they slag if they overheat.");
                ui.label("Iodine decays to xenon (poison pit). Sensors spark when the pile is critical.");
                ui.label("Fire needs air and dies in water. Sparks travel along wire and fire pumps/heaters/TNT.");
                ui.label("Steam pressure shoves fluids around the coolant loop.");
            });
    }
}
