mod keyboard;
mod keyboard_layout;
mod scene;

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use core::ops::RangeInclusive;

use egui::{style::Margin, Frame, Label, Visuals};

use rfd::FileDialog;

use crate::{
    audio_playback::SimpleTemporaryPlayer,
    midi::{InRamMIDIFile, MIDIFileBase, MIDIFileUnion},
};

use self::{keyboard::GuiKeyboard, scene::GuiRenderScene};

use super::{GuiRenderer, GuiState};
use crate::settings::{WasabiPermanentSettings, WasabiTemporarySettings};

struct Fps(VecDeque<Instant>);

const FPS_WINDOW: f64 = 0.5;

impl Fps {
    fn new() -> Self {
        Self(VecDeque::new())
    }

    fn update(&mut self) {
        self.0.push_back(Instant::now());
        while let Some(front) = self.0.front() {
            if front.elapsed().as_secs_f64() > FPS_WINDOW {
                self.0.pop_front();
            } else {
                break;
            }
        }
    }

    fn get_fps(&self) -> f64 {
        if self.0.is_empty() {
            0.0
        } else {
            self.0.len() as f64 / self.0.front().unwrap().elapsed().as_secs_f64()
        }
    }
}

pub struct GuiWasabiWindow {
    render_scene: GuiRenderScene,
    keyboard_layout: keyboard_layout::KeyboardLayout,
    keyboard: GuiKeyboard,
    midi_file: Option<MIDIFileUnion>,
    fps: Fps,
}

impl GuiWasabiWindow {
    pub fn new(renderer: &mut GuiRenderer) -> GuiWasabiWindow {
        GuiWasabiWindow {
            render_scene: GuiRenderScene::new(renderer),
            keyboard_layout: keyboard_layout::KeyboardLayout::new(&Default::default()),
            keyboard: GuiKeyboard::new(),
            midi_file: None,
            fps: Fps::new(),
        }
    }

    /// Defines the layout of our UI
    pub fn layout(
        &mut self,
        state: &mut GuiState,
        perm_settings: &mut WasabiPermanentSettings,
        temp_settings: &mut WasabiTemporarySettings,
    ) {
        let ctx = state.gui.context();

        let window_size = vec![ctx.available_rect().width(), ctx.available_rect().height()];

        self.fps.update();

        ctx.set_visuals(Visuals::dark());

        let note_speed = perm_settings.note_speed;

        // Settings window
        if temp_settings.settings_visible {
            let _settings_frame = Frame::default()
                .inner_margin(egui::style::Margin::same(10.0))
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 170))
                .rounding(egui::Rounding::same(2.0));

            egui::Window::new("Settings")
                .resizable(true)
                .collapsible(true)
                .title_bar(true)
                .scroll2([false, true])
                .enabled(true)
                .open(&mut temp_settings.settings_visible)
                .show(&ctx, |ui| {
                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("SFZ Path: ");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut perm_settings.sfz_path));
                                if ui.button("Browse...").clicked() {
                                    let midi_path: String = FileDialog::new()
                                        .add_filter("sfz", &["sfz"])
                                        .set_directory("/")
                                        .pick_file()
                                        .unwrap()
                                        .into_os_string()
                                        .into_string()
                                        .unwrap();
                                    perm_settings.sfz_path = midi_path;
                                }
                            });
                            ui.end_row();

                            ui.label("Note speed: ");
                            ui.spacing_mut().slider_width = 150.0;
                            ui.add(
                                egui::Slider::new(&mut perm_settings.note_speed, 2.0..=0.001)
                                    .show_value(false),
                            );
                            ui.end_row();

                            ui.label("Background Color: ");
                            ui.color_edit_button_srgba(&mut perm_settings.bg_color);
                            ui.end_row();

                            ui.label("Bar Color: ");
                            ui.color_edit_button_srgba(&mut perm_settings.bar_color);
                            ui.end_row();

                            ui.label("Random Track Colors: ");
                            ui.checkbox(&mut perm_settings.random_colors, "");
                            ui.end_row();

                            ui.label("Keyboard Range: ");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut perm_settings.first_key)
                                        .speed(1)
                                        .clamp_range(RangeInclusive::new(0, 255)),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut perm_settings.last_key)
                                        .speed(1)
                                        .clamp_range(RangeInclusive::new(0, 255)),
                                );
                            });
                            ui.end_row();
                        });
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if ui.button("Save").clicked() {
                            perm_settings.save_to_file();
                        }
                    });
                });
        }

        // Render the top panel
        let panel_height = if temp_settings.panel_visible {
            let panel_height = 40.0;
            let panel_frame = Frame::default()
                .inner_margin(egui::style::Margin::same(10.0))
                .fill(egui::Color32::from_rgb(42, 42, 42));
            egui::TopBottomPanel::top("Top panel")
                .height_range(panel_height..=panel_height)
                .frame(panel_frame)
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Open MIDI").clicked() {
                            let midi_path = FileDialog::new()
                                .add_filter("midi", &["mid"])
                                .set_directory("/")
                                .pick_file();

                            if let Some(midi_path) = midi_path {
                                let mut midi_file =
                                    MIDIFileUnion::InRam(InRamMIDIFile::load_from_file(
                                        &midi_path.into_os_string().into_string().unwrap(),
                                        SimpleTemporaryPlayer::new(&perm_settings.sfz_path),
                                        perm_settings.random_colors,
                                    ));
                                midi_file.timer_mut().play();
                                self.midi_file = Some(midi_file);
                            }
                        }
                        if self.midi_file.is_some() && ui.button("Close MIDI").clicked() {
                            self.midi_file = None;
                        }
                        if ui.button("Play").clicked() {
                            self.midi_file.as_mut().unwrap().timer_mut().play();
                        }
                        if ui.button("Pause").clicked() {
                            self.midi_file.as_mut().unwrap().timer_mut().pause();
                        }
                        if ui.button("Settings").clicked() {
                            match temp_settings.settings_visible {
                                true => temp_settings.settings_visible = false,
                                false => temp_settings.settings_visible = true,
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Note speed: ");
                            ui.add(
                                egui::Slider::new(&mut perm_settings.note_speed, 2.0..=0.001)
                                    .show_value(false),
                            );
                        })
                    });

                    if self.midi_file.is_some() {
                        if let Some(length) = self.midi_file.as_ref().unwrap().midi_length() {
                            let time = self
                                .midi_file
                                .as_ref()
                                .unwrap()
                                .timer()
                                .get_time()
                                .as_secs_f64();
                            let mut progress = time / length;
                            let progress_prev = progress;
                            let slider =
                                egui::Slider::new(&mut progress, 0.0..=1.0).show_value(false);
                            ui.spacing_mut().slider_width = window_size[0] - 20.0;
                            ui.add(slider);
                            if progress_prev != progress {
                                let position = Duration::from_secs_f64(progress * length);
                                self.midi_file.as_mut().unwrap().timer_mut().seek(position);
                            }
                        }
                    } else {
                        let mut progress = 0.0;
                        let slider = egui::Slider::new(&mut progress, 0.0..=1.0).show_value(false);
                        ui.spacing_mut().slider_width = window_size[0] - 20.0;
                        ui.add(slider);
                    }
                });
            panel_height + 20.0
        } else {
            0.0
        };

        // Calculate available space left for keyboard and notes
        // We must render notes before keyboard because the notes
        // renderer tells us the key colors
        let available = ctx.available_rect();
        let height = available.height();
        let visible_keys = std::ops::Range {
            start: perm_settings.first_key,
            end: perm_settings.last_key + 1,
        }
        .len();
        let keyboard_height = 11.6 / visible_keys as f32 * available.width() as f32;
        let notes_height = height - keyboard_height;

        let key_view = self
            .keyboard_layout
            .get_view_for_keys(perm_settings.first_key, perm_settings.last_key);

        let no_frame = Frame::default()
            .inner_margin(Margin::same(0.0))
            .fill(perm_settings.bg_color);

        let stats_frame = Frame::default()
            .inner_margin(egui::style::Margin::same(10.0))
            .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 170))
            .rounding(egui::Rounding::same(5.0));

        let mut render_result_data = None;

        if self.midi_file.is_some() {
            let stats = self.midi_file.as_ref().unwrap().stats();

            // Render the notes
            egui::TopBottomPanel::top("Note panel")
                .height_range(notes_height..=notes_height)
                .frame(no_frame)
                .show(&ctx, |ui| {
                    let one_sec = Duration::from_secs(1);
                    let _five_sec = Duration::from_secs(5);
                    let time = self.midi_file.as_mut().unwrap().timer().get_time();
                    let events = ui.input().events.clone();
                    for event in &events {
                        if let egui::Event::Key { key, pressed, .. } = event {
                            if pressed == &true {
                                match key {
                                    egui::Key::ArrowRight => self
                                        .midi_file
                                        .as_mut()
                                        .unwrap()
                                        .timer_mut()
                                        .seek(time + one_sec),
                                    egui::Key::ArrowLeft => self
                                        .midi_file
                                        .as_mut()
                                        .unwrap()
                                        .timer_mut()
                                        .seek(time - one_sec),
                                    egui::Key::Space => {
                                        self.midi_file.as_mut().unwrap().timer_mut().toggle_pause()
                                    }
                                    egui::Key::F => match temp_settings.panel_visible {
                                        true => temp_settings.panel_visible = false,
                                        false => temp_settings.panel_visible = true,
                                    },
                                    egui::Key::G => match temp_settings.stats_visible {
                                        true => temp_settings.stats_visible = false,
                                        false => temp_settings.stats_visible = true,
                                    },
                                    _ => {}
                                }
                            }
                        }
                    }
                    let result = self.render_scene.draw(
                        state,
                        ui,
                        &key_view,
                        self.midi_file.as_mut().unwrap(),
                        note_speed,
                    );
                    render_result_data = Some(result);
                });
            let render_result_data = render_result_data.unwrap();

            // Render the keyboard
            egui::TopBottomPanel::top("Keyboard panel")
                .height_range(keyboard_height..=keyboard_height)
                .frame(no_frame)
                .show(&ctx, |ui| {
                    self.keyboard.draw(
                        ui,
                        &key_view,
                        &render_result_data.key_colors,
                        &perm_settings.bar_color,
                    );
                });

            // Render the stats
            if temp_settings.stats_visible {
                egui::Window::new("Stats")
                    .resizable(false)
                    .collapsible(false)
                    .title_bar(false)
                    .scroll2([false, false])
                    .enabled(true)
                    .frame(stats_frame)
                    .fixed_pos(egui::Pos2::new(10.0, panel_height + 10.0))
                    .show(&ctx, |ui| {
                        if let Some(length) = self.midi_file.as_mut().unwrap().midi_length() {
                            let time = self
                                .midi_file
                                .as_mut()
                                .unwrap()
                                .timer()
                                .get_time()
                                .as_secs();
                            let time_sec = time % 60;
                            let time_min = (time / 60) % 60;
                            let length_u64 = length as u64;
                            let length_sec = length_u64 % 60;
                            let length_min = (length_u64 / 60) % 60;
                            if time > length_u64 {
                                ui.add(Label::new(format!(
                                    "Time: {:0width$}:{:0width$}/{:0width$}:{:0width$}",
                                    length_min,
                                    length_sec,
                                    length_min,
                                    length_sec,
                                    width = 2
                                )));
                            } else {
                                ui.add(Label::new(format!(
                                    "Time: {:0width$}:{:0width$}/{:0width$}:{:0width$}",
                                    time_min,
                                    time_sec,
                                    length_min,
                                    length_sec,
                                    width = 2
                                )));
                            }
                        }
                        ui.add(Label::new(format!("FPS: {}", self.fps.get_fps().round())));
                        ui.add(Label::new(format!("Total Notes: {}", stats.total_notes)));
                        //ui.add(Label::new(format!("Passed: {}", -1)));  // TODO
                        //ui.add(Label::new(format!("Voice Count: {}", self.synth.as_mut().unwrap().get_voice_count())));
                        ui.add(Label::new(format!(
                            "Rendered: {}",
                            render_result_data.notes_rendered
                        )));
                    });
            }
        } else {
            // Render the notes
            egui::TopBottomPanel::top("Note panel")
                .height_range(notes_height..=notes_height)
                .frame(no_frame)
                .show(&ctx, |_| {});

            // Render the keyboard
            egui::TopBottomPanel::top("Keyboard panel")
                .height_range(keyboard_height..=keyboard_height)
                .frame(no_frame)
                .show(&ctx, |ui| {
                    self.keyboard
                        .draw_empty(ui, &key_view, &perm_settings.bar_color);
                });

            // Render the stats
            if temp_settings.stats_visible {
                egui::Window::new("Stats")
                    .resizable(false)
                    .collapsible(false)
                    .title_bar(false)
                    .scroll2([false, false])
                    .enabled(true)
                    .frame(stats_frame)
                    .fixed_pos(egui::Pos2::new(10.0, panel_height + 10.0))
                    .show(&ctx, |ui| {
                        ui.add(Label::new("Time: 00:00/00:00"));
                        ui.add(Label::new(format!("FPS: {}", self.fps.get_fps().round())));
                        ui.add(Label::new("Total Notes: 0"));
                        ui.add(Label::new("Rendered:0"));
                    });
            }
        }
    }
}
