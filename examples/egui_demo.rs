use std::time::Duration;

use asciiquarium_rust::{
    get_all_fish_assets, update_aquarium, AquariumState, AsciiquariumTheme, AsciiquariumWidget,
    FishInstance,
};
use eframe::egui;
// Use a tiny deterministic LCG inside `spawn_random_fish` instead of `rand`.

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Asciiquarium egui demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

struct MyApp {
    assets: Vec<asciiquarium_rust::FishArt>,
    state: AquariumState,
    theme: AsciiquariumTheme,
    // Controls repaint cadence (ms). Simulation uses an internal dt; higher values reduce CPU and may not linearly affect perceived speed.
    frame_ms: u64,
    bg_enabled: bool,
}

impl MyApp {
    fn new() -> Self {
        let assets = get_all_fish_assets();

        // Choose an initial grid size. You can make this dynamic later if desired.
        let size = (80usize, 24usize);

        let mut state = AquariumState {
            size,
            fishes: Vec::new(),
            ..Default::default()
        };

        // Seed with a few random fish
        for _ in 0..6 {
            spawn_random_fish(&mut state, assets.len());
        }

        // Add a single crab asset on startup (search curated assets for crab)
        spawn_crab(&mut state, &assets);

        let theme = AsciiquariumTheme {
            text_color: egui::Color32::from_rgb(180, 220, 255),
            background: Some(egui::Color32::from_rgb(8, 12, 16)),
            wrap: false,
            enable_color: true,
            palette: None,
        };

        Self {
            assets,
            state,
            theme,
            frame_ms: 50,
            bg_enabled: true,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drive animation based on a simple frame duration.
        update_aquarium(&mut self.state, &self.assets);
        ctx.request_repaint_after(Duration::from_millis(self.frame_ms));

        egui::TopBottomPanel::top("top_controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Grid:");
                // Keep grid integers reasonable. Avoid sliders for usize to reduce friction.
                if ui.button("-W").clicked() && self.state.size.0 > 10 {
                    self.state.size.0 -= 2;
                }
                if ui.button("+W").clicked() {
                    self.state.size.0 += 2;
                }
                if ui.button("-H").clicked() && self.state.size.1 > 5 {
                    self.state.size.1 -= 1;
                }
                if ui.button("+H").clicked() {
                    self.state.size.1 += 1;
                }

                ui.separator();

                ui.label("Frame (ms):");
                ui.label(format!("{} ms", self.frame_ms));
                ui.small("Render cadence; simulation uses fixed dt ~33ms");
                if ui.button("-").clicked() && self.frame_ms > 5 {
                    self.frame_ms -= 2;
                }
                if ui.button("+").clicked() && self.frame_ms < 1000 {
                    self.frame_ms += 2;
                }

                ui.separator();

                ui.label("Theme:");
                ui.color_edit_button_srgba(&mut self.theme.text_color);
                ui.checkbox(&mut self.bg_enabled, "Background");
                if self.bg_enabled {
                    // Ensure background stays Some when enabled
                    if self.theme.background.is_none() {
                        self.theme.background = Some(egui::Color32::from_rgb(8, 12, 16));
                    }
                    if let Some(bg) = &mut self.theme.background {
                        ui.color_edit_button_srgba(bg);
                    }
                } else {
                    self.theme.background = None;
                }

                // Colorized rendering toggle and palette controls
                ui.checkbox(&mut self.theme.enable_color, "Color");
                if self.theme.enable_color {
                    if self.theme.palette.is_none() {
                        self.theme.palette = Some(
                            asciiquarium_rust::widgets::asciiquarium::AsciiquariumPalette {
                                water: egui::Color32::from_rgb(120, 180, 255),
                                water_trail: egui::Color32::from_rgba_unmultiplied(
                                    120, 180, 255, 120,
                                ),
                                seaweed: egui::Color32::from_rgb(60, 180, 120),
                                castle: egui::Color32::from_rgb(200, 200, 200),
                                ship: egui::Color32::from_rgb(230, 230, 230),
                                bubble: egui::Color32::from_rgb(200, 230, 255),
                                shark: egui::Color32::from_rgb(180, 200, 210),
                                whale: egui::Color32::from_rgb(160, 190, 210),
                                fish: egui::Color32::from_rgb(255, 200, 120),
                            },
                        );
                    }
                    if let Some(p) = &mut self.theme.palette {
                        ui.separator();
                        ui.label("Palette:");
                        ui.horizontal(|ui| {
                            ui.label("Water");
                            ui.color_edit_button_srgba(&mut p.water);
                            ui.label("Trail");
                            ui.color_edit_button_srgba(&mut p.water_trail);
                            ui.label("Seaweed");
                            ui.color_edit_button_srgba(&mut p.seaweed);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Bubble");
                            ui.color_edit_button_srgba(&mut p.bubble);
                            ui.label("Fish");
                            ui.color_edit_button_srgba(&mut p.fish);
                            ui.label("Shark");
                            ui.color_edit_button_srgba(&mut p.shark);
                            ui.label("Whale");
                            ui.color_edit_button_srgba(&mut p.whale);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Ship");
                            ui.color_edit_button_srgba(&mut p.ship);
                            ui.label("Castle");
                            ui.color_edit_button_srgba(&mut p.castle);
                        });
                    }
                }

                ui.separator();

                if ui.button("Add fish").clicked() {
                    spawn_random_fish(&mut self.state, self.assets.len());
                }
                if ui.button("Add crab").clicked() {
                    spawn_crab(&mut self.state, &self.assets);
                }
                if ui.button("Reset").clicked() {
                    self.state.fishes.clear();
                    for _ in 0..6 {
                        spawn_random_fish(&mut self.state, self.assets.len());
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Render widget as a single monospace label
            ui.add(AsciiquariumWidget {
                state: &self.state,
                assets: &self.assets,
                theme: &self.theme,
            });
        });
    }
}

fn spawn_random_fish(state: &mut AquariumState, asset_count: usize) {
    if asset_count == 0 {
        return;
    }
    // Tiny deterministic LCG for example RNG (avoids depending on `rand` features).
    let mut s: u64 = 0x1234_5678_9ABC_DEF0u64 ^ (asset_count as u64);
    let mut next_u32 = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        (s >> 32) as u32
    };

    let idx = (next_u32() as usize) % asset_count;

    // Random position within grid; update() will clamp on edges using asset size
    let max_x = if state.size.0 > 0 { state.size.0 - 1 } else { 0 };
    let max_y = if state.size.1 > 0 { state.size.1 - 1 } else { 0 };
    let x = (next_u32() as usize % (max_x + 1)) as f32;
    let y = (next_u32() as usize % (max_y + 1)) as f32;

    // Classic Asciiquarium pacing: horizontal speed 2.5..22.5 cps, minimal vertical drift
    let r = (next_u32() as f32) / (u32::MAX as f32);
    let speed = 2.5_f32 + r * (22.5_f32 - 2.5_f32);
    let dir = if (next_u32() & 1) == 0 { -1.0 } else { 1.0 };
    let vx = dir * speed;
    let mut vy = -0.6_f32 + (next_u32() as f32) / (u32::MAX as f32) * 1.2_f32;
    if vy.abs() < 0.05 {
        vy = 0.0;
    }

    state.fishes.push(FishInstance {
        fish_art_index: idx,
        position: (x, y),
        velocity: (vx, vy),
    });
    state
        .fish_behaviors
        .push(asciiquarium_rust::widgets::asciiquarium::FishBehavior::Normal);
}

fn spawn_crab(state: &mut AquariumState, assets: &[asciiquarium_rust::FishArt]) {
    if assets.is_empty() {
        return;
    }
    // Try to find the curated crab asset by a distinctive substring; fall back to the last asset.
    let idx = assets
        .iter()
        .position(|a| a.art.contains("__^_^__") || a.art.contains("o o"))
        .unwrap_or(assets.len() - 1);
    // Ensure crab is positioned fully on-screen (account for asset height/width)
    let art = &assets[idx];
    let fw = art.width;
    let fh = art.height;
    let x = if state.size.0 > fw { (state.size.0 - fw) / 2 } else { 0 };
    // Place crab near bottom but keep it visible
    let y = if state.size.1 > fh { state.size.1 - fh } else { 0 };

    // Alternate initial horizontal direction so multiple crabs don't all go same way
    let dir = if (state.tick + state.fishes.len() as u64) % 2 == 0 { 1.0 } else { -1.0 };
    let speed = 6.0_f32 * dir; // characters/sec multiplier handled by update

    state.fishes.push(FishInstance {
        fish_art_index: idx,
        position: (x as f32, y as f32),
        velocity: (speed, 0.0),
    });
    state
        .fish_behaviors
        .push(asciiquarium_rust::widgets::asciiquarium::FishBehavior::Normal);
}
