use eframe::egui;
use reqwest::Client;
use serde_json::Value;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

#[derive(Clone, Copy, Debug)]
pub enum Source {
    Places,
    Topics,
}

pub async fn fetch(source: Source, query: &str) -> Result<Vec<String>, String> {
    let client = Client::new();
    let endpoint = match source {
        Source::Places => "https://photon.komoot.io/api/",
        Source::Topics => "https://en.wikipedia.org/w/api.php",
    };
    let mut url =
        url::Url::parse(endpoint).map_err(|e| format!("Invalid suggestion endpoint: {e}"))?;
    match source {
        Source::Places => {
            url.query_pairs_mut().extend_pairs([
                ("q", query),
                ("lang", "en"),
                ("limit", "10"),
                ("layer", "city"),
                ("layer", "state"),
                ("layer", "country"),
            ]);
        }
        Source::Topics => {
            url.query_pairs_mut().extend_pairs([
                ("action", "opensearch"),
                ("search", query),
                ("limit", "10"),
                ("namespace", "0"),
                ("format", "json"),
            ]);
        }
    }
    let data: Value = client
        .get(url)
        .header(
            "User-Agent",
            "PoiskContact/0.1 (desktop search autocomplete)",
        )
        .header("Accept-Language", "en")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("Suggestions unavailable: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Suggestions unavailable: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Invalid suggestion response: {e}"))?;
    parse(source, &data)
}

fn parse(source: Source, data: &Value) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    match source {
        Source::Places => {
            let features = data
                .get("features")
                .and_then(Value::as_array)
                .ok_or("Invalid place response")?;
            for feature in features.iter().take(10) {
                let p = &feature["properties"];
                let Some(name) = p["name"].as_str().filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                let mut parts = vec![name];
                for key in ["state", "country"] {
                    if let Some(part) = p[key].as_str().filter(|s| !s.trim().is_empty()) {
                        if !parts.contains(&part) {
                            parts.push(part);
                        }
                    }
                }
                let label = parts.join(", ");
                if !results.contains(&label) {
                    results.push(label);
                }
            }
        }
        Source::Topics => {
            let titles = data
                .get(1)
                .and_then(Value::as_array)
                .ok_or("Invalid topic response")?;
            for title in titles
                .iter()
                .take(10)
                .filter_map(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                if !results.iter().any(|s| s == title) {
                    results.push(title.to_string());
                }
            }
        }
    }
    Ok(results)
}

#[derive(Default)]
pub struct Autocomplete {
    query: String,
    due: Option<Instant>,
    task: Option<JoinHandle<()>>,
    receiver: Option<Receiver<Result<Vec<String>, String>>>,
    results: Vec<String>,
    error: Option<String>,
    open: bool,
    selected: usize,
}

impl Autocomplete {
    #[cfg(debug_assertions)]
    pub fn review(&mut self, query: &str) {
        self.reset(query, Instant::now() - Duration::from_secs(1));
        self.open = true;
    }

    #[cfg(debug_assertions)]
    pub fn capture_ready(&self) -> bool {
        self.due.is_none()
            && self.receiver.is_none()
            && (!self.results.is_empty() || self.error.is_some())
    }

    fn reset(&mut self, query: &str, now: Instant) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.receiver = None;
        self.query = query.to_string();
        self.results.clear();
        self.error = None;
        self.selected = 0;
        self.due = (query.trim().chars().count() >= 2).then_some(now + Duration::from_millis(500));
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut String,
        hint: &str,
        source: Source,
        runtime: &tokio::runtime::Runtime,
    ) {
        let edit_id = egui::Id::new(format!("autocomplete-{source:?}"));
        let mut chosen = None;
        let mut moved = false;
        if self.open && ui.memory(|memory| memory.has_focus(edit_id)) {
            // Consume dropdown keys before TextEdit uses them for cursor movement.
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.open = false;
            }
            if !self.results.is_empty() {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                    self.selected = (self.selected + 1).min(self.results.len() - 1);
                    moved = true;
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                    self.selected = self.selected.saturating_sub(1);
                    moved = true;
                }
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                chosen = self.results.get(self.selected).cloned();
                self.open = false;
            }
            if moved {
                ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            }
        }
        let response = ui
            .horizontal(|ui| {
                let mut response = ui.add(
                    egui::TextEdit::singleline(value)
                        .id(edit_id)
                        .hint_text(hint)
                        .margin(egui::Margin::symmetric(10, 8))
                        .desired_width(
                            (ui.available_width() - 36.0 - ui.spacing().item_spacing.x).max(60.0),
                        ),
                );
                let arrow = ui
                    .add(egui::Button::new("").min_size(egui::vec2(36.0, response.rect.height())))
                    .on_hover_text("Show online suggestions");
                arrow.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        "Show online suggestions",
                    )
                });
                let mut icon = arrow.clone();
                icon.rect =
                    egui::Rect::from_center_size(arrow.rect.center(), egui::vec2(12.0, 10.0));
                egui::collapsing_header::paint_default_icon(ui, 1.0, &icon);
                response.rect.max.x = arrow.rect.max.x;
                if arrow.clicked() {
                    self.open = !self.open;
                    response.request_focus();
                    if self.open && self.receiver.is_none() && self.results.is_empty() {
                        self.reset(value, Instant::now());
                    }
                }
                response
            })
            .inner;
        if response.changed() || (response.gained_focus() && self.query != *value) {
            self.reset(value, Instant::now());
            self.open = true;
        }
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(results) => self.results = results,
                        Err(error) => self.error = Some(error),
                    }
                    self.receiver = None;
                    self.task = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.error =
                        Some("Suggestion request interrupted. Type again to retry.".into());
                    self.receiver = None;
                    self.task = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.open && self.due.is_some_and(|due| Instant::now() >= due) {
            self.due = None;
            let query = self.query.trim().to_string();
            let (tx, rx) = channel();
            self.receiver = Some(rx);
            let ctx = ui.ctx().clone();
            self.task = Some(runtime.spawn(async move {
                let _ = tx.send(fetch(source, &query).await);
                ctx.request_repaint();
            }));
        }
        if let Some(due) = self.due {
            ui.ctx()
                .request_repaint_after(due.saturating_duration_since(Instant::now()));
        }
        egui::Popup::from_response(&response)
            .open_bool(&mut self.open)
            .width(response.rect.width())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_max_width(response.rect.width());
                ui.label(
                    egui::RichText::new(match source {
                        Source::Places => "LOCATIONS",
                        Source::Topics => "SUGGESTIONS",
                    })
                    .small()
                    .weak(),
                );
                if matches!(source, Source::Places)
                    && ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new(("Anywhere — worldwide", egui::Atom::grow()))
                                .frame_when_inactive(false),
                        )
                        .clicked()
                {
                    chosen = Some(String::new());
                    ui.close();
                }
                if self.receiver.is_some() || self.due.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading suggestions...");
                    });
                } else if let Some(error) = &self.error {
                    ui.label(
                        egui::RichText::new(
                            "Suggestions unavailable. You can still type any value.",
                        )
                        .color(ui.visuals().warn_fg_color),
                    )
                    .on_hover_text(error);
                } else if self.results.is_empty() {
                    ui.label(if self.query.trim().chars().count() < 2 {
                        "Type at least 2 characters for suggestions."
                    } else {
                        "No suggestions found. Keep your own text."
                    });
                }
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for (index, suggestion) in self.results.iter().enumerate() {
                            let (title, detail) = if matches!(source, Source::Places) {
                                suggestion.split_once(", ").unwrap_or((suggestion, ""))
                            } else {
                                (suggestion.as_str(), "")
                            };
                            let mut text = egui::text::LayoutJob::default();
                            text.append(
                                title,
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::TextStyle::Body.resolve(ui.style()),
                                    color: ui.visuals().text_color(),
                                    ..Default::default()
                                },
                            );
                            if !detail.is_empty() {
                                text.append(
                                    &format!("\n{detail}"),
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::TextStyle::Small.resolve(ui.style()),
                                        color: ui.visuals().weak_text_color(),
                                        ..Default::default()
                                    },
                                );
                            }
                            let row = ui
                                .scope(|ui| {
                                    ui.visuals_mut().selection.bg_fill =
                                        ui.visuals().faint_bg_color;
                                    ui.visuals_mut().selection.stroke =
                                        egui::Stroke::new(1.0, ui.visuals().text_color());
                                    ui.add_sized(
                                        [ui.available_width(), 0.0],
                                        egui::Button::selectable(
                                            index == self.selected,
                                            (text, egui::Atom::grow()),
                                        )
                                        .wrap(),
                                    )
                                })
                                .inner
                                .on_hover_text(suggestion);
                            if index == self.selected {
                                if moved {
                                    row.scroll_to_me(Some(egui::Align::Center));
                                }
                                ui.painter().vline(
                                    row.rect.left() + 2.0,
                                    row.rect.y_range().shrink(6.0),
                                    egui::Stroke::new(
                                        2.0,
                                        if ui.visuals().dark_mode {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().hyperlink_color
                                        },
                                    ),
                                );
                            }
                            if row.clicked() {
                                chosen = Some(suggestion.clone());
                                ui.close();
                            }
                        }
                    });
                ui.separator();
                ui.label(
                    egui::RichText::new("Up / Down to navigate · Enter to select · Esc to close")
                        .small()
                        .weak(),
                );
                ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                ui.horizontal_wrapped(|ui| match source {
                    Source::Places => {
                        ui.hyperlink_to("Photon", "https://photon.komoot.io");
                        ui.hyperlink_to(
                            "© OpenStreetMap contributors",
                            "https://www.openstreetmap.org/copyright",
                        );
                    }
                    Source::Topics => {
                        ui.hyperlink_to("Wikipedia topic suggestions", "https://en.wikipedia.org");
                    }
                });
            });
        if !self.open {
            self.due = None;
            if let Some(task) = self.task.take() {
                task.abort();
            }
            self.receiver = None;
        }
        if let Some(chosen) = chosen {
            *value = chosen;
            self.reset(value, Instant::now());
            self.due = None;
            self.open = false;
            response.request_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_parser_disambiguates_cities_and_deduplicates_countries() {
        let data = serde_json::json!({"features": [
            {"properties": {"name": "London", "state": "England", "country": "United Kingdom"}},
            {"properties": {"name": "London", "state": "Ontario", "country": "Canada"}},
            {"properties": {"name": "Germany", "country": "Germany"}},
            {"properties": {"name": "Germany", "country": "Germany"}},
            {"properties": {}}, {"properties": {"name": ""}}
        ]});
        assert_eq!(
            parse(Source::Places, &data).unwrap(),
            [
                "London, England, United Kingdom",
                "London, Ontario, Canada",
                "Germany"
            ]
        );
        assert!(parse(Source::Places, &serde_json::json!({"error":"rate limited"})).is_err());
    }

    #[test]
    fn topic_parser_handles_empty_and_invalid_responses() {
        assert_eq!(
            parse(
                Source::Topics,
                &serde_json::json!(["dent", ["Dentistry", "Dentistry", "Dental implant", ""]])
            )
            .unwrap(),
            ["Dentistry", "Dental implant"]
        );
        assert!(
            parse(Source::Topics, &serde_json::json!(["unknown", []]))
                .unwrap()
                .is_empty()
        );
        assert!(parse(Source::Topics, &serde_json::json!({})).is_err());
    }

    #[test]
    fn new_input_cancels_old_results_and_debounces_unicode() {
        let now = Instant::now();
        let mut state = Autocomplete::default();
        let (tx, rx) = channel();
        state.receiver = Some(rx);
        state.results = vec!["Old location".into()];
        state.reset("Lon", now);
        assert!(tx.send(Ok(vec!["Stale result".into()])).is_err());
        assert!(state.results.is_empty());
        assert_eq!(state.due, Some(now + Duration::from_millis(500)));
        state.reset("М", now);
        assert!(state.due.is_none());
    }

    #[test]
    fn keyboard_selection_and_escape_keep_free_input() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        for (key, expected) in [
            (egui::Key::Enter, "London, England, United Kingdom"),
            (egui::Key::Escape, "Lon"),
        ] {
            let ctx = egui::Context::default();
            let mut value = "Lon".to_string();
            let mut state = Autocomplete {
                query: value.clone(),
                results: vec!["London, England, United Kingdom".into()],
                due: Some(Instant::now() + Duration::from_secs(10)),
                open: true,
                ..Default::default()
            };
            for frame in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(900.0, 600.0),
                        )),
                        events: if frame == 2 {
                            vec![egui::Event::Key {
                                key,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            }]
                        } else {
                            vec![]
                        },
                        ..Default::default()
                    },
                    |ui| {
                        if frame == 0 {
                            ui.memory_mut(|memory| {
                                memory.request_focus(egui::Id::new("autocomplete-Places"))
                            });
                        }
                        state.ui(ui, &mut value, "City", Source::Places, &runtime);
                    },
                );
                output.textures_delta.clear();
            }
            assert_eq!(value, expected);
            assert!(!state.open);
            assert!(state.due.is_none());
        }
    }

    #[test]
    fn keyboard_reaches_suggestions_beyond_the_visible_rows() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let ctx = egui::Context::default();
        let mut value = "Lon".to_string();
        let mut state = Autocomplete {
            query: value.clone(),
            results: (0..10)
                .map(|n| format!("London {n}, England, United Kingdom"))
                .collect(),
            due: Some(Instant::now() + Duration::from_secs(10)),
            open: true,
            ..Default::default()
        };
        for frame in 0..11 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 600.0),
                    )),
                    events: if frame == 0 {
                        vec![]
                    } else {
                        vec![egui::Event::Key {
                            key: if frame == 10 {
                                egui::Key::Enter
                            } else {
                                egui::Key::ArrowDown
                            },
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    },
                    ..Default::default()
                },
                |ui| {
                    if frame == 0 {
                        ui.memory_mut(|memory| {
                            memory.request_focus(egui::Id::new("autocomplete-Places"))
                        });
                    }
                    state.ui(ui, &mut value, "City", Source::Places, &runtime);
                },
            );
            output.textures_delta.clear();
        }
        assert_eq!(value, "London 9, England, United Kingdom");
        assert!(!state.open);
    }

    #[tokio::test]
    #[ignore = "uses live autocomplete providers"]
    async fn live_city_country_and_topic_suggestions() {
        let cities = fetch(Source::Places, "Krasnoyarsk").await.unwrap();
        assert!(
            cities.iter().any(|s| s.starts_with("Krasnoyarsk,")),
            "{cities:?}"
        );
        let countries = fetch(Source::Places, "Germ").await.unwrap();
        assert!(countries.iter().any(|s| s == "Germany"), "{countries:?}");
        let topics = fetch(Source::Topics, "dent").await.unwrap();
        assert!(topics.iter().any(|s| s == "Dentistry"), "{topics:?}");
        println!("Cities: {cities:?}\nCountries: {countries:?}\nTopics: {topics:?}");
    }
}
