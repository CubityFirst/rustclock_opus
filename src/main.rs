#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Datelike, Local, Timelike};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 560.0])
            .with_min_inner_size([360.0, 420.0])
            .with_title("Rust Clock"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Clock",
        options,
        Box::new(|cc| Ok(Box::new(ClockApp::new(cc)))),
    )
}

/// A single alarm: a time of day, an optional label, and whether it is armed.
#[derive(Clone, Serialize, Deserialize)]
struct Alarm {
    hour: u32,
    minute: u32,
    label: String,
    enabled: bool,
    /// Date (ordinal-day key) on which this alarm last fired, so it only
    /// rings once per day even though we check many times per second.
    #[serde(skip)]
    last_fired_day: Option<i32>,
}

impl Alarm {
    fn time_string(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Serialize, Deserialize, Default)]
struct PersistedState {
    alarms: Vec<Alarm>,
    use_24h: bool,
}

struct ClockApp {
    alarms: Vec<Alarm>,
    use_24h: bool,

    // New-alarm input fields.
    new_hour: u32,
    new_minute: u32,
    new_label: String,

    // Currently ringing alarm (index into `alarms`) and a flash phase counter.
    ringing: Option<usize>,
    flash_phase: f32,
}

impl ClockApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let state = load_state();
        let now = Local::now();
        Self {
            alarms: state.alarms,
            use_24h: state.use_24h,
            new_hour: now.hour(),
            new_minute: (now.minute() + 1) % 60,
            new_label: String::new(),
            ringing: None,
            flash_phase: 0.0,
        }
    }

    fn save(&self) {
        let state = PersistedState {
            alarms: self.alarms.clone(),
            use_24h: self.use_24h,
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            if let Some(path) = state_path() {
                let _ = std::fs::write(path, json);
            }
        }
    }

    /// Check every alarm against the current time; arm `self.ringing` if one
    /// should fire. An alarm fires at the start of its minute, at most once
    /// per calendar day.
    fn check_alarms(&mut self, now: DateTime<Local>) {
        if self.ringing.is_some() {
            return; // already ringing; ignore further triggers until dismissed
        }
        let day_key = now.num_days_from_ce();
        for i in 0..self.alarms.len() {
            let a = &self.alarms[i];
            if a.enabled
                && a.hour == now.hour()
                && a.minute == now.minute()
                && a.last_fired_day != Some(day_key)
            {
                self.alarms[i].last_fired_day = Some(day_key);
                self.ringing = Some(i);
                break;
            }
        }
    }
}

impl eframe::App for ClockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep the UI live so the clock ticks and alarms are checked.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        let now = Local::now();
        self.check_alarms(now);

        if self.ringing.is_some() {
            self.flash_phase += ctx.input(|i| i.stable_dt);
            self.show_ringing(ctx, now);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_clock(ui, now);
            ui.separator();
            self.draw_new_alarm(ui);
            ui.separator();
            self.draw_alarm_list(ui);
        });
    }
}

impl ClockApp {
    fn draw_clock(&self, ui: &mut egui::Ui, now: DateTime<Local>) {
        ui.add_space(8.0);
        let time_text = if self.use_24h {
            now.format("%H:%M:%S").to_string()
        } else {
            now.format("%I:%M:%S %p").to_string()
        };
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(time_text)
                    .size(56.0)
                    .strong()
                    .monospace(),
            );
            ui.label(
                egui::RichText::new(now.format("%A, %e %B %Y").to_string())
                    .size(16.0)
                    .weak(),
            );
        });
        ui.add_space(6.0);
    }

    fn draw_new_alarm(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("New alarm").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(self.use_24h, "24h")
                    .on_hover_text("Toggle 24-hour clock")
                    .clicked()
                {
                    self.use_24h = !self.use_24h;
                    self.save();
                }
            });
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.new_hour)
                    .range(0..=23)
                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
            );
            ui.label(":");
            ui.add(
                egui::DragValue::new(&mut self.new_minute)
                    .range(0..=59)
                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
            );
            ui.text_edit_singleline(&mut self.new_label);
            if ui.button("➕ Add").clicked() {
                self.alarms.push(Alarm {
                    hour: self.new_hour,
                    minute: self.new_minute,
                    label: self.new_label.trim().to_string(),
                    enabled: true,
                    last_fired_day: None,
                });
                self.new_label.clear();
                self.alarms.sort_by_key(|a| (a.hour, a.minute));
                self.save();
            }
        });
    }

    fn draw_alarm_list(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Alarms").strong());
        ui.add_space(4.0);

        if self.alarms.is_empty() {
            ui.weak("No alarms set. Add one above.");
            return;
        }

        let mut remove: Option<usize> = None;
        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, alarm) in self.alarms.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut alarm.enabled, "").changed() {
                        changed = true;
                    }
                    let time = alarm.time_string();
                    let text = egui::RichText::new(time).size(22.0).monospace();
                    ui.label(if alarm.enabled {
                        text.strong()
                    } else {
                        text.weak()
                    });
                    if !alarm.label.is_empty() {
                        ui.label(&alarm.label);
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button("🗑").on_hover_text("Delete").clicked() {
                                remove = Some(i);
                            }
                        },
                    );
                });
                ui.separator();
            }
        });

        if let Some(i) = remove {
            self.alarms.remove(i);
            changed = true;
        }
        if changed {
            self.save();
        }
    }

    fn show_ringing(&mut self, ctx: &egui::Context, _now: DateTime<Local>) {
        let Some(idx) = self.ringing else { return };
        let alarm = self.alarms.get(idx).cloned();
        let Some(alarm) = alarm else {
            self.ringing = None;
            return;
        };

        // Flashing background colour driven by the phase counter.
        let pulse = (self.flash_phase * 6.0).sin() * 0.5 + 0.5;
        let bg = egui::Color32::from_rgb(
            (180.0 + 60.0 * pulse) as u8,
            (40.0 * (1.0 - pulse)) as u8,
            (40.0 * (1.0 - pulse)) as u8,
        );

        egui::Window::new("⏰ Alarm")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .frame(egui::Frame::popup(&ctx.style()).fill(bg))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(alarm.time_string())
                            .size(48.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    let label = if alarm.label.is_empty() {
                        "Alarm!".to_string()
                    } else {
                        alarm.label.clone()
                    };
                    ui.label(
                        egui::RichText::new(label)
                            .size(22.0)
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(12.0);
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Dismiss").size(20.0),
                        ))
                        .clicked()
                    {
                        self.ringing = None;
                        self.flash_phase = 0.0;
                    }
                    ui.add_space(8.0);
                });
            });

        // Make sure the window keeps repainting while flashing.
        ctx.request_repaint();
    }
}

fn state_path() -> Option<PathBuf> {
    let mut dir = dirs_config_dir()?;
    dir.push("rustclock");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("alarms.json");
    Some(dir)
}

/// Minimal config-dir resolver without pulling in the `dirs` crate.
fn dirs_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

fn load_state() -> PersistedState {
    state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
