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
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for id in app.palette.elements_filtered() {
            let name = aura_lite_elements::registry::name_for_id(id);
            let color = aura_lite_elements::registry::color_for_id(id);
            let is_selected = app.palette.selected_id == id;
            let btn_text = format!("{} ({})", name, id);
            let mut btn = egui::Button::new(btn_text);
            if is_selected {
                btn = btn.fill(egui::Color32::from_rgb(80, 80, 200));
            }
            if ui.add(btn).clicked() {
                app.set_selected_element(id);
            }
            // show color preview
            ui.horizontal(|ui| {
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
    });
    ui.label(format!("Current tool: {:?}", app.tools.brush.tool));
    ui.add(egui::Slider::new(&mut app.tools.brush.temperature, 0..=5000).text("Temperature"));
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
            app.simulation.grid.clear();
        }
    });
    ui.add(egui::Slider::new(&mut app.controller.speed, 0.1..=5.0).text("Speed"));
    ui.add(egui::Slider::new(&mut app.controller.tick_rate, 1..=120).text("Tick rate"));
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
                let _ = aura_lite_io::save_to_file(
                    &path,
                    &app.simulation.grid,
                    &app.simulation.settings,
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
}
