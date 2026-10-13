//! Window › Timeline: a floating panel that creates and drives the document timeline. All actions
//! run the engine `timeline.*` commands, so the control channel drives it the same way.

use egui::{RichText, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::theme::Tokens;

/// Timeline panel view state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TimelineUi {
    pub open: bool,
    /// Playing back (auto-advancing the playhead at the frame rate).
    #[serde(default)]
    pub playing: bool,
    /// egui time (seconds) of the last auto-advance.
    #[serde(default, skip)]
    pub last_time: f64,
    #[serde(default)]
    pub onion: bool,
}

/// Menu checkmark for Window › Timeline.
pub fn checked(app: &PhotocraftApp, id: &str) -> Option<bool> {
    (id == "window.panel.timeline").then_some(app.ui.timeline.open)
}

/// Ids this module owns (for parity / enablement).
pub fn handles(id: &str) -> bool {
    id == "window.panel.timeline"
}

/// Toggle the Timeline panel.
pub fn menu(app: &mut PhotocraftApp, id: &str, _params: &Value) -> Option<Result<Value, String>> {
    if id == "window.panel.timeline" {
        app.ui.timeline.open = !app.ui.timeline.open;
        return Some(Ok(json!({ "open": app.ui.timeline.open })));
    }
    None
}

/// Render the Timeline panel.
pub fn windows(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if !app.ui.timeline.open {
        return;
    }
    let tl = app.session.active().and_then(|d| d.doc.timeline.clone());
    let have_doc = app.session.active().is_some();
    let mut act: Option<(&str, Value)> = None;
    let mut close = false;
    let mut toggle_play = false;
    let playing = app.ui.timeline.playing;
    let mut onion = app.ui.timeline.onion;
    let t = Tokens::get(ctx);

    crate::analysis_ui::panel_window(app, ctx, "timeline", tl!("Timeline"), vec2(0.0, 520.0), 660.0, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tl!("Timeline")).strong().color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::icons::button(ui, "x", 20.0, false, tl!("Close")).clicked() {
                    close = true;
                }
            });
        });
        ui.separator();

        if !have_doc {
            ui.weak(tl!("Open a document to use the timeline."));
            return;
        }
        match &tl {
            None => {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if crate::widgets::primary_button(ui, tl!("Create drawing animation"), 200.0).clicked() {
                        act = Some(("timeline.create", json!({"duration":1,"fps":12.0})));
                    }
                    ui.add_space(4.0);
                    ui.weak(tl!("Draw frame by frame · 12 fps"));
                });
            }
            Some(tl) => {
                let active_video = app.session.active().and_then(|d| d.active_layer.and_then(|id| d.doc.layer(id))).is_some_and(|l| l.video.is_some());
                ui.horizontal_wrapped(|ui| {
                    if !active_video && ui.button("Add drawing layer").clicked() {
                        act = Some(("layer.videoLayers.newBlankVideoLayer", json!({})));
                    }
                    if active_video {
                        for (label, cmd) in [
                            ("Blank frame", "layer.videoLayers.insertBlankFrame"),
                            ("Duplicate frame", "layer.videoLayers.duplicateFrame"),
                            ("Delete frame", "layer.videoLayers.deleteFrame"),
                        ] {
                            if ui.button(label).clicked() {
                                act = Some((cmd, json!({})));
                            }
                        }
                    }
                    ui.checkbox(&mut onion, "Onion skin");
                    let mut fps = tl.fps;
                    if ui.add(egui::DragValue::new(&mut fps).range(1.0..=60.0).suffix(" fps")).changed() {
                        act = Some(("timeline.setProps", json!({"fps":fps})));
                    }
                    if ui.button("Export animation…").clicked() {
                        act = Some(("file.export.renderVideo", json!({})));
                    }
                });
                ui.horizontal(|ui| {
                    let play_icon = if playing { "pause" } else { "play" };
                    if crate::icons::button(ui, play_icon, 22.0, playing, if playing { tl!("Pause") } else { tl!("Play") }).clicked() {
                        toggle_play = true;
                    }
                    if crate::icons::button(ui, "chevron-left", 22.0, false, tl!("Previous frame")).clicked() {
                        act = Some(("timeline.previousFrame", json!({})));
                    }
                    ui.label(
                        RichText::new(crate::i18n::fmt(
                            crate::i18n::t("Frame {current} / {total}"),
                            &[("current", &(tl.current + 1).to_string()), ("total", &tl.duration.to_string())],
                        ))
                        .color(t.text)
                        .monospace(),
                    );
                    if crate::icons::button(ui, "chevron-right", 22.0, false, tl!("Next frame")).clicked() {
                        act = Some(("timeline.nextFrame", json!({})));
                    }
                    ui.separator();
                    ui.label(RichText::new(format!("{:.2}s", tl.time())).color(t.text_dim).monospace());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::icons::button(ui, "trash", 20.0, false, tl!("Delete Timeline")).clicked() {
                            act = Some(("timeline.delete", json!({})));
                        }
                        ui.label(RichText::new(format!("{:.0} fps", tl.fps)).color(t.text_dim));
                    });
                });
                ui.add_space(6.0);
                // Scrubber: frame playhead across the duration.
                if tl.duration > 1 {
                    let mut frame = tl.current as f64;
                    let resp = ui.add(egui::Slider::new(&mut frame, 0.0..=(tl.duration - 1) as f64).integer().show_value(false).text("playhead"));
                    if resp.changed() {
                        act = Some(("timeline.setFrame", json!({ "frame": frame as u64 })));
                    }
                }
            }
        }
    });

    app.ui.timeline.onion = onion;
    if close {
        app.ui.timeline.open = false;
        app.ui.timeline.playing = false;
    }
    if toggle_play {
        app.ui.timeline.playing = !app.ui.timeline.playing;
        app.ui.timeline.last_time = ctx.input(|i| i.time);
    }
    if let Some((cmd, p)) = act {
        let previous = app.session.active().and_then(|d| d.doc.timeline.as_ref()).map_or(0, |t| t.current);
        if let Err(e) = if cmd == "file.export.renderVideo" { app.export_animation() } else { app.run(cmd, p) } {
            app.ui.status = e;
        } else {
            if cmd == "timeline.create"
                && let Err(e) = app.run("layer.videoLayers.newBlankVideoLayer", json!({}))
            {
                app.ui.status = e;
            }
            if matches!(cmd, "layer.videoLayers.insertBlankFrame" | "layer.videoLayers.duplicateFrame")
                && let Err(e) = app.run("timeline.setFrame", json!({"frame":previous.saturating_add(1)}))
            {
                app.ui.status = e;
            }
        }
    }
    // Playback: auto-advance the playhead at the frame rate (loops).
    if app.ui.timeline.playing {
        if let Some(t) = app.session.active().and_then(|d| d.doc.timeline.clone()) {
            let now = ctx.input(|i| i.time);
            let period = 1.0 / f64::from(t.fps.max(1.0));
            if now - app.ui.timeline.last_time >= period {
                let elapsed = ((now - app.ui.timeline.last_time) / period).floor().max(1.) as usize;
                let next = t.current.saturating_add(elapsed) % t.duration.max(1);
                app.ui.timeline.last_time = now;
                if let Err(e) = app.run("timeline.setFrame", json!({"frame":next as u64})) {
                    app.ui.status = e;
                    app.ui.timeline.playing = false;
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(period));
        } else {
            app.ui.timeline.playing = false;
        }
    }
}
