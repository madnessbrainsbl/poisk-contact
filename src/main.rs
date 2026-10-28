#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod autocomplete;
mod chrome_crawler;
mod dkim;
mod mailer;
mod models;
mod search;
mod storage;

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use mailer::{
    MailerProgress, analyze_template, build_transport, run_followup_mailer, run_mailer,
    send_direct_mx, send_single_email, test_direct_mx_connectivity, verify_mailbox_rcpt,
};
use models::{
    ContactRecord, DnsblResult, DomainAuditResult, EmailTemplate, LeadScore, LogEntry,
    SearchFilter, SendStatus, SmtpConfig, SpamCheckResult, SuppressionList,
};
use search::{SearchProgress, run_hybrid_search};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use storage::{
    export_contacts_to_amocrm_csv, export_contacts_to_bitrix24_csv, export_contacts_to_csv,
    export_contacts_to_instantly_csv, export_contacts_to_lemlist_csv, export_contacts_to_vcf,
    import_external_contacts_file, load_app_settings, load_contacts_from_json,
    load_suppression_list, save_app_settings, save_contacts_to_json, save_suppression_list,
};

// Clean Motion-inspired minimalist dark theme palette; layout dimensions are logical points.
const BG_COLOR: Color32 = Color32::from_rgb(9, 9, 11);
const CARD_BG: Color32 = Color32::from_rgb(18, 18, 21);
const CARD_BG_HOVER: Color32 = Color32::from_rgb(24, 24, 28);
const SIDEBAR_BG: Color32 = Color32::from_rgb(12, 12, 14);
const BORDER_COLOR: Color32 = Color32::from_rgb(39, 39, 42);
// Outline of interactive controls: 3.2:1 on CARD_BG and BG_COLOR (WCAG 1.4.11 needs 3:1).
const CONTROL_BORDER: Color32 = Color32::from_rgb(100, 100, 108);
const BORDER_SUBTLE: Color32 = Color32::from_rgb(31, 31, 35);
const ACCENT_TEAL: Color32 = Color32::from_rgb(34, 197, 94);
const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
const ACCENT_BLUE: Color32 = Color32::from_rgb(96, 165, 250);
const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
const ACCENT_ROSE: Color32 = Color32::from_rgb(239, 68, 68);
const ACTION_FILL: Color32 = Color32::WHITE;
const ACTION_HOVER: Color32 = Color32::from_rgb(228, 228, 231);
const TEXT_COLOR: Color32 = Color32::from_rgb(250, 250, 250);
const TEXT_DARK: Color32 = Color32::from_rgb(9, 9, 11);
const TEXT_MUTED: Color32 = Color32::from_rgb(161, 161, 170);
const TEXT_FAINT: Color32 = Color32::from_rgb(113, 113, 122);
const PAGE_MARGIN: f32 = 24.0;
const SECTION_GAP: f32 = 20.0;
const CONTROL_HEIGHT: f32 = 32.0;
const COMPACT_WIDTH: f32 = 880.0;
const NAV_WIDTH: f32 = 196.0;

#[derive(PartialEq, Clone, Copy, Debug)]
enum ActiveTab {
    Search,
    Contacts,
    Template,
    Smtp,
    Logs,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ContactSortField {
    Default,
    Company,
    Email,
    Score,
}

const WORKSPACE_TABS: [(ActiveTab, &str); 5] = [
    (ActiveTab::Search, "Find contacts"),
    (ActiveTab::Contacts, "Contacts"),
    (ActiveTab::Template, "Email template"),
    (ActiveTab::Smtp, "Mail and sending"),
    (ActiveTab::Logs, "Activity log"),
];

struct AppState {
    filter: SearchFilter,
    place_suggestions: autocomplete::Autocomplete,
    topic_suggestions: autocomplete::Autocomplete,
    smtp: SmtpConfig,
    template: EmailTemplate,
    contacts: Vec<ContactRecord>,
    logs: Vec<LogEntry>,

    active_tab: ActiveTab,
    is_searching: bool,
    is_mailing: bool,

    search_rx: Option<Receiver<SearchProgress>>,
    automatic_business_collection: bool,
    all_industries: bool,
    collection_cancel: Arc<AtomicBool>,
    collection_backup_created: bool,
    search_initial_count: usize,
    search_collected_count: usize,
    search_save_error: Option<String>,
    database_load_error: Option<String>,
    mailer_rx: Option<Receiver<MailerProgress>>,

    tokio_rt: tokio::runtime::Runtime,
    default_json_path: PathBuf,
    default_csv_path: PathBuf,
    settings_path: PathBuf,
    suppression_path: PathBuf,
    suppression_list: SuppressionList,

    table_search_query: String,
    table_score_filter: String,
    table_category_filter: String,
    table_status_filter: String,
    only_phone_filter: bool,
    only_tg_filter: bool,
    only_pending_filter: bool,
    only_rcpt_filter: bool,
    selected_contact_id: Option<String>,
    url_crawl_input: String,
    chrome_crawler_input: String,
    test_email_target: String,
    status_message: String,
    total_sent: usize,
    total_failed: usize,

    // Deliverability Audit & Filters
    audit_domain_input: String,
    sender_audit_result: Option<(DomainAuditResult, DnsblResult)>,
    is_auditing_sender: bool,
    audit_rx: Option<Receiver<(DomainAuditResult, DnsblResult)>>,
    spam_check_result: Option<SpamCheckResult>,
    spintax_samples: Vec<(String, String)>,
    editor_preview_key: String,
    editor_preview_subject: String,
    editor_preview_body: String,
    template_notice: Option<(bool, String)>,
    only_personal_filter: bool,
    hide_catch_all_filter: bool,
    pending_delete: Option<String>,
    confirm_clear: bool,
    log_level_filter: String,
    log_search_query: String,
    quick_segment_filter: Option<String>,
    active_toast: Option<(String, Instant, bool)>,
    contact_sort: (ContactSortField, bool),
    log_autoscroll: bool,
    show_shortcuts_modal: bool,
    show_command_palette: bool,
    command_palette_query: String,
    command_palette_selected: usize,
    preview_contact_index: usize,
    compact_density: bool,
    quick_outreach_contact_id: Option<String>,
    quick_outreach_subject: String,
    quick_outreach_body: String,
}

impl AppState {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_ui(&cc.egui_ctx);
        let app = Self::load(Path::new("."));
        #[cfg(debug_assertions)]
        let app = {
            let mut app = app;
            if let Ok(review) = std::env::var("POISK_UI_REVIEW") {
                app.active_tab = match review.split(':').next().unwrap_or_default() {
                    "contacts" | "inspector" => ActiveTab::Contacts,
                    "template" => ActiveTab::Template,
                    "smtp" => ActiveTab::Smtp,
                    "logs" => ActiveTab::Logs,
                    _ => ActiveTab::Search,
                };
                if review.starts_with("inspector") {
                    app.selected_contact_id = app.contacts.first().map(|c| c.id.clone());
                }
                if review.starts_with("template") && review.contains("saved") {
                    app.save_template();
                }
                if review.contains("zoom") {
                    cc.egui_ctx.set_zoom_factor(1.5);
                }
                if review.starts_with("logs") && review.contains("sample") {
                    app.is_searching = true;
                    app.search_collected_count = 58;
                    app.filter.max_items = 100;
                    app.add_log(
                        "SUCCESS",
                        "Collected 58/100 companies; contacts saved automatically.",
                    );
                    app.add_log("WARN", &format!("Search provider cooldown: https://example.org/search?q={}. Saved progress will resume on the next run.", "%D0%BA%D1%80".repeat(40)));
                    app.add_log("INFO", "Continuing with the next available source.");
                }
                if review.contains("places") {
                    app.filter.city = "Krasnoy".into();
                    app.place_suggestions.review(&app.filter.city);
                } else if review.contains("countries") {
                    app.filter.city = "Germ".into();
                    app.place_suggestions.review(&app.filter.city);
                } else if review.contains("topics") {
                    app.filter.stack = "dent".into();
                    app.topic_suggestions.review(&app.filter.stack);
                }
            }
            app
        };
        app
    }

    fn load(directory: &Path) -> Self {
        let default_json_path = directory.join("contacts.json");
        let default_csv_path = directory.join("contacts.csv");
        let settings_path = directory.join("settings.json");
        let suppression_path = directory.join("suppression.json");
        let suppression_list = load_suppression_list(&suppression_path);

        let (contacts, database_load_error) = match load_contacts_from_json(&default_json_path) {
            Ok(contacts) => (contacts, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let options = std::fs::read(directory.join("collection-options.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let mut filter = options
            .as_ref()
            .and_then(|options| serde_json::from_value(options["filter"].clone()).ok())
            .unwrap_or_else(SearchFilter::default);
        if options.is_none() {
            filter.max_items = 100;
        }
        let (smtp, template) = load_app_settings(&settings_path)
            .unwrap_or((SmtpConfig::default(), EmailTemplate::default()));

        let tokio_rt = tokio::runtime::Runtime::new().expect("Failed to start Tokio runtime");

        let mut app = Self {
            filter,
            place_suggestions: autocomplete::Autocomplete::default(),
            topic_suggestions: autocomplete::Autocomplete::default(),
            smtp,
            template,
            contacts,
            logs: Vec::new(),
            active_tab: ActiveTab::Search,
            is_searching: false,
            is_mailing: false,
            search_rx: None,
            automatic_business_collection: options
                .as_ref()
                .and_then(|v| v["automatic"].as_bool())
                .unwrap_or(true),
            all_industries: options
                .as_ref()
                .and_then(|v| v["all_industries"].as_bool())
                .unwrap_or(true),
            collection_cancel: Arc::new(AtomicBool::new(false)),
            collection_backup_created: false,
            search_initial_count: 0,
            search_collected_count: 0,
            search_save_error: None,
            database_load_error,
            mailer_rx: None,
            tokio_rt,
            default_json_path,
            default_csv_path,
            settings_path,
            suppression_path,
            suppression_list,
            table_search_query: String::new(),
            table_score_filter: "All".to_string(),
            table_category_filter: "All".to_string(),
            table_status_filter: "All".to_string(),
            only_phone_filter: false,
            only_tg_filter: false,
            only_pending_filter: false,
            only_rcpt_filter: false,
            selected_contact_id: None,
            url_crawl_input: String::new(),
            chrome_crawler_input: String::new(),
            test_email_target: "".to_string(),
            status_message: "Ready to collect contacts.".to_string(),
            total_sent: 0,
            total_failed: 0,

            audit_domain_input: "yandex.ru".to_string(),
            sender_audit_result: None,
            is_auditing_sender: false,
            audit_rx: None,
            spam_check_result: None,
            spintax_samples: Vec::new(),
            editor_preview_key: String::new(),
            editor_preview_subject: String::new(),
            editor_preview_body: String::new(),
            template_notice: None,
            only_personal_filter: false,
            hide_catch_all_filter: false,
            pending_delete: None,
            confirm_clear: false,
            log_level_filter: "All".to_string(),
            log_search_query: String::new(),
            quick_segment_filter: None,
            active_toast: None,
            contact_sort: (ContactSortField::Default, false),
            log_autoscroll: true,
            show_shortcuts_modal: false,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            preview_contact_index: 0,
            compact_density: false,
            quick_outreach_contact_id: None,
            quick_outreach_subject: String::new(),
            quick_outreach_body: String::new(),
        };

        app.add_log(
            "INFO",
            "Application started. Search, DNS verification and mailer modules are ready.",
        );
        app
    }

    fn apply_sort(&mut self) {
        match self.contact_sort.0 {
            ContactSortField::Score => {
                let score_rank = |c: &ContactRecord| -> u16 {
                    let base = match c.score {
                        LeadScore::Hot => 300,
                        LeadScore::Warm => 200,
                        LeadScore::Cold => 100,
                    };
                    base + c.lead_score.min(99) as u16
                };
                if self.contact_sort.1 {
                    self.contacts.sort_by_key(score_rank);
                } else {
                    self.contacts
                        .sort_by(|a, b| score_rank(b).cmp(&score_rank(a)));
                }
            }
            ContactSortField::Company => {
                if self.contact_sort.1 {
                    self.contacts
                        .sort_by(|a, b| a.company.to_lowercase().cmp(&b.company.to_lowercase()));
                } else {
                    self.contacts
                        .sort_by(|a, b| b.company.to_lowercase().cmp(&a.company.to_lowercase()));
                }
            }
            ContactSortField::Email => {
                if self.contact_sort.1 {
                    self.contacts
                        .sort_by(|a, b| a.email.to_lowercase().cmp(&b.email.to_lowercase()));
                } else {
                    self.contacts
                        .sort_by(|a, b| b.email.to_lowercase().cmp(&a.email.to_lowercase()));
                }
            }
            ContactSortField::Default => {}
        }
    }

    fn show_toast(&mut self, text: impl Into<String>, is_success: bool) {
        self.active_toast = Some((text.into(), Instant::now(), is_success));
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        if let Some((msg, time, is_success)) = &self.active_toast {
            if time.elapsed() > Duration::from_millis(2600) {
                self.active_toast = None;
                return;
            }
            let screen_rect = ctx.content_rect();
            let toast_pos = egui::pos2(
                (screen_rect.right() - 280.0).max(12.0),
                (screen_rect.bottom() - 62.0).max(12.0),
            );
            egui::Area::new(egui::Id::new("floating_toast"))
                .fixed_pos(toast_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 18, 22))
                        .stroke(Stroke::new(1.0, BORDER_COLOR))
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(12, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (dot_color, symbol) = if *is_success {
                                    (Color32::from_rgb(74, 222, 128), "●")
                                } else {
                                    (Color32::from_rgb(248, 113, 113), "●")
                                };
                                ui.label(RichText::new(symbol).color(dot_color).small());
                                ui.label(RichText::new(msg).color(Color32::WHITE).size(12.5));
                            });
                        });
                });
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn render_shortcuts_modal(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts_modal {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(18.0),
            )
            .show(ctx, |ui| {
                ui.set_width(480.0);
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("Keyboard Shortcuts")
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_kbd(ui, "Esc");
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Power-user hotkeys for navigation, discovery, and outreach.")
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.add_space(14.0);

                ui.columns(2, |cols| {
                    cols[0].label(
                        RichText::new("NAVIGATION")
                            .small()
                            .strong()
                            .color(TEXT_FAINT),
                    );
                    cols[0].add_space(6.0);
                    for (keys, desc) in [
                        ("Ctrl 1..5", "Switch workspace tabs"),
                        ("Ctrl K", "Open command palette"),
                        ("/", "Focus contact search"),
                        ("Ctrl F", "Search in table"),
                        ("Alt D", "Toggle table density"),
                        ("Esc", "Close modal / drawer"),
                        ("F1", "Toggle shortcuts cheatsheet"),
                    ] {
                        cols[0].horizontal(|ui| {
                            render_kbd(ui, keys);
                            ui.label(
                                RichText::new(desc)
                                    .small()
                                    .color(Color32::from_rgb(228, 228, 231)),
                            );
                        });
                        cols[0].add_space(4.0);
                    }

                    cols[1].label(
                        RichText::new("ACTIONS & OUTREACH")
                            .small()
                            .strong()
                            .color(TEXT_FAINT),
                    );
                    cols[1].add_space(6.0);
                    for (keys, desc) in [
                        ("Ctrl S", "Save template locally"),
                        ("Click Header", "Sort table by column"),
                        ("Enter", "Open lead in drawer"),
                        ("Shuffle", "Sample spintax branch"),
                        ("Pause", "Lock log auto-scroll"),
                    ] {
                        cols[1].horizontal(|ui| {
                            render_kbd(ui, keys);
                            ui.label(
                                RichText::new(desc)
                                    .small()
                                    .color(Color32::from_rgb(228, 228, 231)),
                            );
                        });
                        cols[1].add_space(4.0);
                    }
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        self.show_shortcuts_modal = false;
                    }
                });
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_shortcuts_modal = false;
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        let mut open = true;
        let mut triggered_action: Option<&'static str> = None;

        let commands: [(
            &'static str,
            &'static str,
            &'static str,
            Option<&'static str>,
        ); 24] = [
            ("nav_search", "Navigation", "Find Contacts", Some("Ctrl 1")),
            (
                "nav_contacts",
                "Navigation",
                "Contacts Grid",
                Some("Ctrl 2"),
            ),
            (
                "nav_template",
                "Navigation",
                "Email Template",
                Some("Ctrl 3"),
            ),
            ("nav_smtp", "Navigation", "Mail & Sending", Some("Ctrl 4")),
            ("nav_logs", "Navigation", "Activity Log", Some("Ctrl 5")),
            ("export_csv", "Data & Exports", "Export leads to CSV", None),
            (
                "export_vcf",
                "Data & Exports",
                "Export leads to vCard (.vcf)",
                None,
            ),
            (
                "export_amocrm",
                "Data & Exports",
                "Export leads to AmoCRM",
                None,
            ),
            (
                "export_bitrix24",
                "Data & Exports",
                "Export leads to Bitrix24",
                None,
            ),
            (
                "export_instantly",
                "Data & Exports",
                "Export leads to Instantly",
                None,
            ),
            (
                "export_lemlist",
                "Data & Exports",
                "Export leads to Lemlist",
                None,
            ),
            ("db_save", "Data & Exports", "Save database to JSON", None),
            ("db_load", "Data & Exports", "Load database from JSON", None),
            ("db_merge", "Data & Exports", "Merge duplicate leads", None),
            (
                "toggle_density",
                "Data & Exports",
                "Toggle table density (Compact / Relaxed)",
                Some("Alt+D"),
            ),
            (
                "coll_start",
                "Outreach & Collection",
                "Start business collection",
                None,
            ),
            (
                "coll_stop",
                "Outreach & Collection",
                "Stop collection",
                None,
            ),
            (
                "enrich_waterfall",
                "Outreach & Collection",
                "Waterfall enrich all leads",
                None,
            ),
            (
                "spintax_shuffle",
                "Outreach & Collection",
                "Shuffle spintax variations",
                None,
            ),
            (
                "flipper_next",
                "Outreach & Collection",
                "Preview next lead in envelope",
                None,
            ),
            (
                "tmpl_save",
                "Outreach & Collection",
                "Save email template",
                Some("Ctrl S"),
            ),
            (
                "audit_sender",
                "Outreach & Collection",
                "Audit sender domain",
                None,
            ),
            (
                "log_scroll",
                "Console & System",
                "Toggle log auto-scroll",
                None,
            ),
            (
                "help_shortcuts",
                "Console & System",
                "Open keyboard shortcuts",
                Some("F1"),
            ),
        ];

        let query = self.command_palette_query.trim().to_lowercase();
        let filtered: Vec<_> = commands
            .iter()
            .copied()
            .filter(|(_, cat, title, _)| {
                if query.is_empty() {
                    true
                } else {
                    title.to_lowercase().contains(&query) || cat.to_lowercase().contains(&query)
                }
            })
            .collect();

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if self.command_palette_selected + 1 < filtered.len() {
                self.command_palette_selected += 1;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if self.command_palette_selected > 0 {
                self.command_palette_selected -= 1;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(cmd) = filtered.get(self.command_palette_selected) {
                triggered_action = Some(cmd.0);
            }
        }

        let cmd_palette_id = egui::Id::new("command_palette_input");

        egui::Window::new("Command Palette")
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 72.0))
            .frame(
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 12)),
            )
            .show(ctx, |ui| {
                ui.set_width(480.0);
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::singleline(&mut self.command_palette_query)
                        .id(cmd_palette_id)
                        .frame(egui::Frame::NONE)
                        .desired_width(ui.available_width() - 40.0)
                        .hint_text("Type a command or action...");
                    let resp = ui.add(text_edit);
                    if resp.changed() {
                        self.command_palette_selected = 0;
                    }
                    resp.request_focus();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_kbd(ui, "Esc");
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                if filtered.is_empty() {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("No matching commands")
                            .small()
                            .color(TEXT_FAINT),
                    );
                    ui.add_space(12.0);
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (idx, (id, cat, title, shortcut)) in filtered.iter().enumerate() {
                                let is_selected = idx == self.command_palette_selected;
                                let bg_fill = if is_selected {
                                    Color32::from_rgb(26, 26, 32)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 28.0),
                                    egui::Sense::click(),
                                );
                                if is_selected {
                                    ui.painter().rect_filled(rect, 4.0, bg_fill);
                                    ui.painter().vline(
                                        rect.left() + 1.5,
                                        rect.y_range().shrink(5.0),
                                        Stroke::new(2.0, Color32::from_rgb(96, 165, 250)),
                                    );
                                } else if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        4.0,
                                        Color32::from_rgb(20, 20, 24),
                                    );
                                }

                                if response.clicked() {
                                    triggered_action = Some(id);
                                }

                                let cat_pos = Pos2::new(rect.left() + 10.0, rect.center().y);
                                ui.painter().text(
                                    cat_pos,
                                    egui::Align2::LEFT_CENTER,
                                    *cat,
                                    egui::FontId::proportional(11.0),
                                    TEXT_FAINT,
                                );

                                let title_pos = Pos2::new(rect.left() + 150.0, rect.center().y);
                                let title_color = if is_selected {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(228, 228, 231)
                                };
                                ui.painter().text(
                                    title_pos,
                                    egui::Align2::LEFT_CENTER,
                                    *title,
                                    egui::FontId::proportional(13.0),
                                    title_color,
                                );

                                if let Some(keys) = shortcut {
                                    let right_pos = Pos2::new(rect.right() - 8.0, rect.center().y);
                                    ui.painter().text(
                                        right_pos,
                                        egui::Align2::RIGHT_CENTER,
                                        *keys,
                                        egui::FontId::monospace(10.5),
                                        TEXT_MUTED,
                                    );
                                }
                            }
                        });
                }
            });

        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_command_palette = false;
        }

        if let Some(action) = triggered_action {
            self.show_command_palette = false;
            match action {
                "nav_search" => {
                    self.active_tab = ActiveTab::Search;
                    self.show_toast("Navigated to Find Contacts", true);
                }
                "nav_contacts" => {
                    self.active_tab = ActiveTab::Contacts;
                    self.show_toast("Navigated to Contacts Grid", true);
                }
                "nav_template" => {
                    self.active_tab = ActiveTab::Template;
                    self.show_toast("Navigated to Email Template", true);
                }
                "nav_smtp" => {
                    self.active_tab = ActiveTab::Smtp;
                    self.show_toast("Navigated to Mail & Sending", true);
                }
                "nav_logs" => {
                    self.active_tab = ActiveTab::Logs;
                    self.show_toast("Navigated to Activity Log", true);
                }
                "export_csv" => {
                    let path = self.default_json_path.with_file_name("contacts.csv");
                    match export_contacts_to_csv(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to CSV", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "export_vcf" => {
                    let path = self.default_json_path.with_file_name("contacts.vcf");
                    match export_contacts_to_vcf(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to vCard (.vcf)", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "export_amocrm" => {
                    let path = self.default_json_path.with_file_name("contacts_amocrm.csv");
                    match export_contacts_to_amocrm_csv(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to AmoCRM format", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "export_bitrix24" => {
                    let path = self
                        .default_json_path
                        .with_file_name("contacts_bitrix24.csv");
                    match export_contacts_to_bitrix24_csv(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to Bitrix24 format", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "export_instantly" => {
                    let path = self
                        .default_json_path
                        .with_file_name("contacts_instantly.csv");
                    match export_contacts_to_instantly_csv(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to Instantly format", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "export_lemlist" => {
                    let path = self
                        .default_json_path
                        .with_file_name("contacts_lemlist.csv");
                    match export_contacts_to_lemlist_csv(&self.contacts, &path) {
                        Ok(()) => self.show_toast("Exported contacts to Lemlist format", true),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), false),
                    }
                }
                "db_save" => match save_contacts_to_json(&self.contacts, &self.default_json_path) {
                    Ok(()) => self.show_toast("Contact database saved", true),
                    Err(e) => self.show_toast(format!("Save failed: {e}"), false),
                },
                "db_load" => match load_contacts_from_json(&self.default_json_path) {
                    Ok(loaded) => {
                        self.contacts = loaded;
                        self.selected_contact_id = None;
                        self.show_toast(format!("Loaded {} contacts", self.contacts.len()), true);
                    }
                    Err(e) => self.show_toast(format!("Load failed: {e}"), false),
                },
                "db_merge" => {
                    let (merged, stats) = crate::search::waterfall_merge::deduplicate_and_merge(
                        self.contacts.clone(),
                    );
                    match save_contacts_to_json(&merged, &self.default_json_path) {
                        Ok(()) => {
                            self.contacts = merged;
                            self.show_toast(
                                format!("Merged {} duplicates", stats.duplicates_merged),
                                true,
                            );
                        }
                        Err(e) => self.show_toast(format!("Merge save failed: {e}"), false),
                    }
                }
                "coll_start" => {
                    self.start_search();
                    self.active_tab = ActiveTab::Logs;
                }
                "coll_stop" => {
                    self.collection_cancel.store(true, Ordering::Relaxed);
                    self.show_toast("Stopping collection", true);
                }
                "enrich_waterfall" => {
                    self.start_waterfall_enrichment();
                    self.active_tab = ActiveTab::Logs;
                }
                "spintax_shuffle" => {
                    if let Some(c) = self.contacts.first().cloned() {
                        self.editor_update_preview(&c, true);
                    }
                    self.show_toast("Sample variation generated", true);
                }
                "tmpl_save" => {
                    self.save_template();
                }
                "audit_sender" => {
                    self.start_sender_audit();
                    self.active_tab = ActiveTab::Smtp;
                }
                "log_scroll" => {
                    self.log_autoscroll = !self.log_autoscroll;
                    let st = if self.log_autoscroll {
                        "Auto"
                    } else {
                        "Paused"
                    };
                    self.show_toast(format!("Log scroll: {st}"), true);
                }
                "toggle_density" => {
                    self.compact_density = !self.compact_density;
                    let mode = if self.compact_density {
                        "Compact (+40% rows visible)"
                    } else {
                        "Relaxed"
                    };
                    self.show_toast(format!("Table density: {mode}"), true);
                }
                "flipper_next" => {
                    if !self.contacts.is_empty() {
                        self.preview_contact_index =
                            (self.preview_contact_index + 1) % self.contacts.len();
                        self.active_tab = ActiveTab::Template;
                        if let Some(c) = self.contacts.get(self.preview_contact_index).cloned() {
                            self.editor_update_preview(&c, true);
                            self.show_toast(format!("Previewing lead: {}", c.name), true);
                        }
                    }
                }
                "help_shortcuts" => {
                    self.show_shortcuts_modal = true;
                }
                _ => {}
            }
        }
    }

    fn add_log(&mut self, level: &str, message: &str) {
        self.logs.push(LogEntry::new(level, message));
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    fn start_search(&mut self) {
        if self.is_searching {
            return;
        }
        if let Some(error) = &self.database_load_error {
            self.status_message = format!(
                "The existing database could not be loaded. Collection is disabled to preserve it: {error}"
            );
            self.add_log("ERROR", &self.status_message.clone());
            return;
        }
        if self.automatic_business_collection
            && (self.filter.city.trim().is_empty()
                || (!self.all_industries && self.filter.stack.trim().is_empty()))
        {
            self.status_message =
                "Enter a location, and choose Different industries or enter a business/service."
                    .into();
            self.add_log("WARN", &self.status_message.clone());
            return;
        }
        let directory = self.default_json_path.parent().unwrap_or(Path::new("."));
        let options = serde_json::json!({"automatic": self.automatic_business_collection, "all_industries": self.all_industries, "filter": self.filter});
        if let Err(error) = std::fs::write(
            directory.join("collection-options.json"),
            options.to_string(),
        ) {
            self.add_log(
                "WARN",
                &format!("Collection preferences could not be saved: {error}"),
            );
        }
        self.collection_cancel = Arc::new(AtomicBool::new(false));
        self.collection_backup_created = false;
        self.search_initial_count = self.contacts.len();
        self.search_collected_count = 0;
        self.search_save_error = None;
        self.is_searching = true;
        self.status_message = format!(
            "Search: query '{}', location '{}'...",
            self.filter.stack, self.filter.city
        );

        let (tx, rx) = channel();
        self.search_rx = Some(rx);
        let filter_clone = self.filter.clone();
        let automatic = self.automatic_business_collection;
        let all_industries = self.all_industries;
        let cancel = self.collection_cancel.clone();
        let progress_path = self
            .default_json_path
            .with_file_name("collection-progress.json");

        self.tokio_rt.spawn(async move {
            if automatic {
                search::business::run_business_search(
                    filter_clone,
                    all_industries,
                    Some(progress_path),
                    cancel,
                    tx,
                )
                .await;
            } else {
                run_hybrid_search(filter_clone, tx).await;
            }
        });
    }

    fn save_collected_contacts(&mut self, new_records: Vec<ContactRecord>) {
        let mut combined = self.contacts.clone();
        combined.extend(new_records);
        self.contacts = search::waterfall_merge::deduplicate_and_merge(combined).0;
        if self.database_load_error.is_some() {
            self.search_save_error = self.database_load_error.clone();
            return;
        }
        let saved = (|| -> Result<(), String> {
            if !self.collection_backup_created && self.default_json_path.exists() {
                let backups = self
                    .default_json_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("backups");
                std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
                let backup = backups.join(format!(
                    "contacts-before-collection-{}.json",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ));
                std::fs::copy(&self.default_json_path, backup)
                    .map_err(|e| format!("Database backup failed: {e}"))?;
            }
            self.collection_backup_created = true;
            save_contacts_to_json(&self.contacts, &self.default_json_path)
        })();
        if let Err(error) = saved {
            self.search_save_error = Some(error.clone());
            self.collection_cancel.store(true, Ordering::Relaxed);
            self.status_message =
                format!("Results are in memory, but the database could not be saved: {error}");
            self.add_log("ERROR", &self.status_message.clone());
        } else {
            self.status_message = format!(
                "Collection: {} database contacts; saved automatically.",
                self.contacts.len()
            );
            if let Err(error) = export_contacts_to_csv(&self.contacts, &self.default_csv_path) {
                self.add_log(
                    "WARN",
                    &format!("Database saved, but CSV export failed: {error}"),
                );
            }
        }
    }

    fn mail_settings_ready(&mut self) -> bool {
        if let Err(error) = mailer::validate_delivery_identity(&self.smtp, self.smtp.direct_mx) {
            self.status_message = error.clone();
            self.add_log("ERROR", &error);
            self.active_tab = ActiveTab::Smtp;
            return false;
        }
        true
    }

    fn start_mailing(&mut self) {
        if self.is_mailing {
            return;
        }
        if !self.mail_settings_ready() {
            return;
        }

        let selected_contacts: Vec<ContactRecord> = self
            .contacts
            .iter()
            .filter(|c| {
                c.selected
                    && c.status == SendStatus::Pending
                    && !self.suppression_list.is_suppressed(&c.email)
            })
            .cloned()
            .collect();

        if selected_contacts.is_empty() {
            self.status_message =
                "No contacts selected for mailing, or all recipients are suppressed.".to_string();
            self.add_log("WARN", "The recipient list is empty or suppressed.");
            return;
        }

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        if self.smtp.last_sent_day != today {
            self.smtp.last_sent_day = today;
            self.smtp.sent_today = 0;
        }

        if self.smtp.daily_limit > 0 && self.smtp.sent_today >= self.smtp.daily_limit {
            self.status_message = format!(
                "Daily sending limit reached ({}/{}). Sending stopped.",
                self.smtp.sent_today, self.smtp.daily_limit
            );
            self.add_log(
                "WARN",
                &format!(
                    "Daily sending limit ({}) reached. Adjust it in Mail settings if needed.",
                    self.smtp.daily_limit
                ),
            );
            return;
        }

        if !self.smtp.direct_mx && (self.smtp.host.is_empty() || self.smtp.username.is_empty()) {
            self.status_message =
                "Configure the SMTP server and login in Mail settings.".to_string();
            self.active_tab = ActiveTab::Smtp;
            self.add_log("ERROR", "External SMTP settings are incomplete.");
            return;
        }

        self.is_mailing = true;
        let mode_desc = if self.smtp.direct_mx {
            "direct delivery (Direct MX, port 25)"
        } else {
            "external SMTP relay"
        };
        self.status_message = format!(
            "Mailing started for {} recipients ({})...",
            selected_contacts.len(),
            mode_desc
        );
        self.total_sent = 0;
        self.total_failed = 0;

        let (tx, rx) = channel();
        self.mailer_rx = Some(rx);

        let config_clone = self.smtp.clone();
        let template_clone = self.template.clone();

        self.tokio_rt.spawn(async move {
            run_mailer(config_clone, template_clone, selected_contacts, tx).await;
        });
    }

    fn start_followup_mailing(&mut self) {
        if self.is_mailing {
            return;
        }
        if !self.mail_settings_ready() {
            return;
        }

        let followable_contacts: Vec<ContactRecord> = self
            .contacts
            .iter()
            .filter(|c| {
                c.selected
                    && c.status == SendStatus::Sent
                    && c.follow_up_count >= 1
                    && c.follow_up_count < 3
                    && !self.suppression_list.is_suppressed(&c.email)
            })
            .cloned()
            .collect();

        if followable_contacts.is_empty() {
            self.status_message = "No eligible follow-up recipients. Select previously emailed contacts with 1 or 2 messages.".to_string();
            self.add_log(
                "WARN",
                "The follow-up recipient list is empty or suppressed.",
            );
            return;
        }

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        if self.smtp.last_sent_day != today {
            self.smtp.last_sent_day = today;
            self.smtp.sent_today = 0;
        }

        if self.smtp.daily_limit > 0 && self.smtp.sent_today >= self.smtp.daily_limit {
            self.status_message = format!(
                "Daily sending limit reached ({}/{}). Sending stopped.",
                self.smtp.sent_today, self.smtp.daily_limit
            );
            self.add_log(
                "WARN",
                &format!("Daily sending limit ({}) reached.", self.smtp.daily_limit),
            );
            return;
        }

        if !self.smtp.direct_mx && (self.smtp.host.is_empty() || self.smtp.username.is_empty()) {
            self.status_message =
                "Configure the SMTP server and login in Mail settings.".to_string();
            self.active_tab = ActiveTab::Smtp;
            self.add_log("ERROR", "External SMTP settings are incomplete.");
            return;
        }

        self.is_mailing = true;
        let mode_desc = if self.smtp.direct_mx {
            "direct delivery (Direct MX)"
        } else {
            "external SMTP relay"
        };
        self.status_message = format!(
            "Drip follow-up started for {} recipients ({})...",
            followable_contacts.len(),
            mode_desc
        );
        self.total_sent = 0;
        self.total_failed = 0;

        let (tx, rx) = channel();
        self.mailer_rx = Some(rx);

        let config_clone = self.smtp.clone();
        let template_clone = self.template.clone();

        self.tokio_rt.spawn(async move {
            run_followup_mailer(config_clone, template_clone, followable_contacts, tx).await;
        });
    }

    fn send_test_email(&mut self) {
        if self.is_mailing || !self.mail_settings_ready() {
            return;
        }
        let target = self.test_email_target.trim().to_string();
        if target.parse::<lettre::Address>().is_err() {
            self.add_log("ERROR", "Enter a valid test recipient email.");
            return;
        }

        let dummy_contact = ContactRecord::new(
            "Test recipient",
            &target,
            "Test company",
            "Director",
            &self.filter.city,
            &self.filter.stack,
            "",
            "",
            "Test",
            "",
            LeadScore::Hot,
            true,
            "mx.test.",
        );
        let subject = mailer::render_template(
            &self.template.subject,
            &dummy_contact,
            &self.smtp.sender_name,
        );
        let body =
            mailer::render_template(&self.template.body, &dummy_contact, &self.smtp.sender_name);

        let config_clone = self.smtp.clone();
        let target_clone = target.clone();
        let (tx, rx) = channel();
        self.mailer_rx = Some(rx);
        self.is_mailing = true;

        let mode_str = if config_clone.direct_mx {
            "Direct MX"
        } else {
            "SMTP relay"
        };
        self.add_log(
            "INFO",
            &format!("Sending a test email to {} using {}...", target, mode_str),
        );

        self.tokio_rt.spawn(async move {
            let res = if config_clone.direct_mx {
                send_direct_mx(&target_clone, &config_clone, &subject, &body).await
            } else {
                match build_transport(&config_clone) {
                    Ok(transport) => {
                        send_single_email(&transport, &config_clone, &target_clone, &subject, &body)
                    }
                    Err(e) => Err(e),
                }
            };

            let _ = tx.send(MailerProgress::TestDone(res.map(|_| ())));
        });
    }

    fn open_quick_outreach(&mut self, contact_id: &str) {
        if let Some(contact) = self.contacts.iter().find(|c| c.id == contact_id) {
            let subj =
                mailer::render_template(&self.template.subject, contact, &self.smtp.sender_name);
            let bdy = mailer::render_template(&self.template.body, contact, &self.smtp.sender_name);
            self.quick_outreach_contact_id = Some(contact_id.to_string());
            self.quick_outreach_subject = subj;
            self.quick_outreach_body = bdy;
        }
    }

    fn send_direct_contact_email(&mut self, contact_id: String, subject: String, body: String) {
        if self.is_mailing || !self.mail_settings_ready() {
            return;
        }
        let contact = match self.contacts.iter().find(|c| c.id == contact_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let target = contact.email.trim().to_string();
        if target.parse::<lettre::Address>().is_err() {
            self.show_toast(format!("Invalid recipient email: {target}"), false);
            return;
        }
        if self.suppression_list.is_suppressed(&target) {
            self.show_toast(format!("{target} is in suppression list"), false);
            return;
        }

        let config_clone = self.smtp.clone();
        let target_clone = target.clone();
        let cid = contact_id.clone();
        let (tx, rx) = channel();
        self.mailer_rx = Some(rx);
        self.is_mailing = true;

        let mode_str = if config_clone.direct_mx {
            "Direct MX"
        } else {
            "SMTP relay"
        };
        self.add_log(
            "INFO",
            &format!(
                "Sending direct outreach to {} ({}) using {}...",
                contact.company, target, mode_str
            ),
        );

        let in_reply_to = contact.last_message_id.clone();

        self.tokio_rt.spawn(async move {
            let res = if config_clone.direct_mx {
                mailer::send_direct_mx_with_headers(
                    &target_clone,
                    &config_clone,
                    &subject,
                    &body,
                    in_reply_to.as_deref(),
                )
                .await
            } else {
                match mailer::build_transport(&config_clone) {
                    Ok(transport) => mailer::send_via_relay_with_headers(
                        &transport,
                        &config_clone,
                        &target_clone,
                        &subject,
                        &body,
                        in_reply_to.as_deref(),
                    ),
                    Err(e) => Err(e),
                }
            };

            match res {
                Ok(msg_id) => {
                    let _ = tx.send(MailerProgress::ItemSent {
                        id: cid,
                        email: target_clone,
                        success: true,
                        error_msg: None,
                        current: 1,
                        total: 1,
                        message_id: Some(msg_id),
                    });
                    let _ = tx.send(MailerProgress::Done { sent: 1, failed: 0 });
                }
                Err(err) => {
                    let _ = tx.send(MailerProgress::ItemSent {
                        id: cid,
                        email: target_clone,
                        success: false,
                        error_msg: Some(err),
                        current: 1,
                        total: 1,
                        message_id: None,
                    });
                    let _ = tx.send(MailerProgress::Done { sent: 0, failed: 1 });
                }
            }
        });
    }

    fn test_port_25(&mut self) {
        if self.is_mailing {
            return;
        }
        let (tx, rx) = channel();
        self.mailer_rx = Some(rx);
        self.add_log("INFO", "Checking outbound port 25 (Direct MX)...");

        self.tokio_rt.spawn(async move {
            match test_direct_mx_connectivity().await {
                Ok(msg) => {
                    let _ = tx.send(MailerProgress::Log(
                        "SUCCESS".to_string(),
                        format!("Port 25 is available. Direct MX connectivity works. ({})", msg),
                    ));
                }
                Err(e) => {
                    let _ = tx.send(MailerProgress::Log(
                        "WARN".to_string(),
                        format!("Port 25 check: {}. If your ISP blocks the port, use an external SMTP relay.", e),
                    ));
                }
            }
        });
    }

    fn launch_2gis_web(&mut self) {
        self.add_log("INFO", "Starting the local 2GIS dashboard (port 8666)...");
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "start_2gis_web.bat"])
            .spawn();
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "http://127.0.0.1:8666"])
            .spawn();
        self.status_message = "2GIS dashboard opened (http://127.0.0.1:8666)".to_string();
    }

    fn start_direct_url_crawl(&mut self) {
        let text = self.url_crawl_input.trim();
        if text.is_empty() {
            self.status_message = "Enter website URLs to crawl.".to_string();
            return;
        }

        let urls: Vec<String> = text
            .lines()
            .flat_map(|l| l.split_whitespace())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if urls.is_empty() {
            return;
        }

        self.is_searching = true;
        self.status_message = format!("Direct crawl started for {} websites...", urls.len());
        self.add_log(
            "INFO",
            &format!("Website crawler: processing {} URLs...", urls.len()),
        );

        let (tx, rx) = channel();
        self.search_rx = Some(rx);

        self.tokio_rt.spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(8))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(SearchProgress::Error(format!("HTTP client error: {}", e)));
                    return;
                }
            };

            let verifier = crate::search::doh::MxVerifier::new();
            let mut results = Vec::new();

            for (idx, mut u) in urls.into_iter().enumerate() {
                if !u.starts_with("http://") && !u.starts_with("https://") {
                    u = format!("https://{}", u);
                }
                let _ = tx.send(SearchProgress::Log(
                    "INFO".to_string(),
                    format!("Crawling ({}/...): {}", idx + 1, u),
                ));
                match crate::search::crawler::crawl_website_for_contacts(
                    &client,
                    &u,
                    "Krasnoyarsk",
                    "Website / organization",
                )
                .await
                {
                    Ok(crawled) => {
                        for mut rec in crawled {
                            if !rec.email.is_empty() {
                                let (mx_ok, mx_rec) =
                                    verifier.check_email(&client, &rec.email).await;
                                rec.mx_valid = mx_ok;
                                rec.mx_record = mx_rec;
                            }
                            results.push(rec);
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(SearchProgress::Log(
                            "WARN".to_string(),
                            format!("{}: {}", u, e),
                        ));
                    }
                }
            }

            let _ = tx.send(SearchProgress::Done(results));
        });
    }

    fn start_waterfall_enrichment(&mut self) {
        if self.is_searching {
            return;
        }

        let target_contacts: Vec<ContactRecord> = if self.contacts.iter().any(|c| c.selected) {
            self.contacts
                .iter()
                .filter(|c| c.selected)
                .cloned()
                .collect()
        } else {
            self.contacts.clone()
        };

        if target_contacts.is_empty() {
            self.status_message = "No contacts available for waterfall enrichment.".to_string();
            return;
        }

        self.is_searching = true;
        self.status_message = format!(
            "Waterfall OSINT started for {} contacts...",
            target_contacts.len()
        );
        self.add_log(
            "INFO",
            &format!(
                "Waterfall OSINT: enriching {} contacts (websites, socials, E.164, RCPT 250)...",
                target_contacts.len()
            ),
        );

        let (tx, rx) = channel();
        self.search_rx = Some(rx);

        self.tokio_rt.spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(8))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(SearchProgress::Error(format!("HTTP client error: {}", e)));
                    return;
                }
            };

            let verifier = crate::search::doh::MxVerifier::new();
            let mut enriched_records = Vec::new();
            let total_contacts = target_contacts.len();

            for (idx, mut rec) in target_contacts.into_iter().enumerate() {
                let _ = tx.send(SearchProgress::Log(
                    "INFO".to_string(),
                    format!("Waterfall ({}/{}): {} [{}]", idx + 1, total_contacts, rec.company, rec.email),
                ));

                // 1. Phone intelligence (PhoneInfoga style)
                if !rec.phone.is_empty() {
                    let p_info = crate::search::phone_intel::parse_and_enrich_phone(&rec.phone);
                    if p_info.is_valid {
                        rec.phone = p_info.formatted;
                        rec.phone_carrier = p_info.carrier;
                        rec.phone_region = p_info.region;
                        rec.phone_type = p_info.line_type;
                        rec.whatsapp_url = p_info.whatsapp_url.unwrap_or_default();
                        rec.telegram_url = p_info.telegram_url.unwrap_or_default();
                    }
                }

                // 2. Crawl website if present
                if !rec.url.is_empty() {
                    let mut url = rec.url.clone();
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        url = format!("https://{}", url);
                    }
                    if let Ok(crawled_list) = crate::search::crawler::crawl_website_for_contacts(&client, &url, &rec.city, &rec.company).await {
                        for c in crawled_list {
                            if rec.vk_url.is_empty() && !c.vk_url.is_empty() {
                                rec.vk_url = c.vk_url;
                            }
                            if rec.whatsapp.is_empty() && !c.whatsapp.is_empty() {
                                rec.whatsapp = c.whatsapp;
                            }
                            if rec.inn.is_empty() && !c.inn.is_empty() {
                                rec.inn = c.inn;
                            }
                            if rec.ogrn.is_empty() && !c.ogrn.is_empty() {
                                rec.ogrn = c.ogrn;
                            }
                            if rec.vat_id.is_empty() && !c.vat_id.is_empty() {
                                rec.vat_id = c.vat_id;
                            }
                            if c.hiring_active {
                                rec.hiring_active = true;
                                for r in c.hiring_roles {
                                    if !rec.hiring_roles.contains(&r) {
                                        rec.hiring_roles.push(r);
                                    }
                                }
                            }
                            for tag in c.tags {
                                if !rec.tags.contains(&tag) {
                                    rec.tags.push(tag);
                                }
                            }
                            if rec.email.is_empty() && !c.email.is_empty() {
                                rec.email = c.email;
                                rec.mx_valid = c.mx_valid;
                                rec.mx_record = c.mx_record;
                            }
                        }
                    }
                }

                // 3. Fallback tag detection
                if rec.tags.is_empty() {
                    let combined_text = format!("{} {} {}", rec.company, rec.role, rec.notes);
                    rec.tags = crate::search::extractor::detect_industry_tags(&combined_text);
                }

                // 4. DoH MX check
                if !rec.email.is_empty() && (!rec.mx_valid || rec.mx_record.is_empty()) {
                    let (mx_ok, mx_rec) = verifier.check_email(&client, &rec.email).await;
                    rec.mx_valid = mx_ok;
                    rec.mx_record = mx_rec;
                }

                // 5. Zero-bounce RCPT probe
                if !rec.email.is_empty() && rec.mx_valid && !rec.rcpt_verified {
                    let mx_hint = if !rec.mx_record.is_empty() { Some(rec.mx_record.as_str()) } else { None };
                    match verify_mailbox_rcpt(&rec.email, mx_hint).await {
                        Ok(is_valid) => {
                            rec.rcpt_verified = is_valid;
                            if is_valid {
                                let _ = tx.send(SearchProgress::Log(
                                    "SUCCESS".to_string(),
                                    format!("RCPT 250 OK: server accepted {}.", rec.email),
                                ));
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(SearchProgress::Log(
                                "WARN".to_string(),
                                format!("RCPT probe {}: {}", rec.email, err),
                            ));
                        }
                    }
                }

                // 6. Developer OSINT (GitHub search by email)
                if !rec.email.is_empty() && rec.github_url.is_empty() {
                    let _ = crate::search::github_osint::enrich_contact_with_github(&client, &mut rec).await;
                }

                // 7. Dynamic Icebreaker generation
                rec.icebreaker = crate::search::icebreaker::generate_dynamic_icebreaker(&rec);

                // 8. Catch-All, Provider, Role-Account, Gravatar & Lead Scoring
                if !rec.email.is_empty() {
                    rec.is_role_account = crate::search::extractor::is_role_based_email(&rec.email);
                    rec.is_disposable = crate::search::extractor::is_disposable_email(&rec.email);
                    if rec.mail_provider.is_empty() && !rec.mx_record.is_empty() {
                        rec.mail_provider = crate::search::extractor::detect_mail_provider(&rec.mx_record);
                    }
                    if let Some((_, domain)) = rec.email.split_once('@') {
                        let mx_hint = if !rec.mx_record.is_empty() { Some(rec.mx_record.as_str()) } else { None };
                        if let Ok(is_ca) = crate::mailer::detect_catch_all_domain(domain, mx_hint).await {
                            rec.is_catch_all = is_ca;
                        }
                    }
                    if !rec.gravatar_found && rec.avatar_url.is_empty() {
                        if let Some(avatar) = crate::search::extractor::check_gravatar_profile(&client, &rec.email).await {
                            rec.gravatar_found = true;
                            rec.avatar_url = avatar;
                        }
                    }
                    rec.lead_score = crate::search::extractor::calculate_lead_score(&rec);
                }

                enriched_records.push(rec);
            }

            // Waterfall Deduplication & cluster merging
            let (merged_records, stats) = crate::search::waterfall_merge::deduplicate_and_merge(enriched_records);
            let _ = tx.send(SearchProgress::Log(
                "SUCCESS".to_string(),
                format!(
                    "Waterfall deduplication: merged {} duplicates, enriched {} fields, {} contacts remaining.",
                    stats.duplicates_merged, stats.fields_enriched, stats.final_count
                ),
            ));

            let _ = tx.send(SearchProgress::Done(merged_records));
        });
    }

    fn start_sender_audit(&mut self) {
        if self.is_auditing_sender {
            return;
        }
        let target = if !self.audit_domain_input.trim().is_empty() {
            self.audit_domain_input.trim().to_string()
        } else if let Some((_, d)) = self.smtp.from_email.split_once('@') {
            d.to_string()
        } else if let Some((_, d)) = self.smtp.username.split_once('@') {
            d.to_string()
        } else {
            "yandex.ru".to_string()
        };

        self.is_auditing_sender = true;
        self.add_log(
            "INFO",
            &format!(
                "Auditing sender domain '{}' (SPF/DMARC/MX/DNSBL)...",
                target
            ),
        );

        let (tx, rx) = channel();
        self.audit_rx = Some(rx);

        self.tokio_rt.spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(6))
                .build()
                .unwrap_or_default();
            let verifier = crate::search::doh::MxVerifier::new();
            let audit = crate::mailer::audit_sender_domain(&client, &verifier, &target).await;
            let dnsbl = crate::mailer::check_dnsbl_reputation(&client, &target).await;
            let _ = tx.send((audit, dnsbl));
        });
    }

    fn run_spam_check(&mut self) {
        let check =
            crate::mailer::analyze_spam_triggers(&self.template.subject, &self.template.body);
        self.spam_check_result = Some(check);
        self.add_log("INFO", "Email template spam-trigger analysis complete.");
    }

    fn generate_spintax_samples(&mut self) {
        let mut samples = Vec::new();
        for _ in 0..6 {
            let s = crate::mailer::expand_spintax(&self.template.subject);
            let b = crate::mailer::expand_spintax(&self.template.body);
            samples.push((s, b));
        }
        self.spintax_samples = samples;
    }

    fn import_chrome_crawler(&mut self) {
        if self.is_searching || self.is_mailing {
            return;
        }
        if let Some(error) = &self.database_load_error {
            self.status_message = format!("Import blocked; existing database is invalid: {error}");
            self.add_log("ERROR", &self.status_message.clone());
            return;
        }
        match chrome_crawler::parse_contacts(&self.chrome_crawler_input) {
            Ok(records) => {
                let count = records.len();
                self.collection_backup_created = false;
                self.search_save_error = None;
                self.save_collected_contacts(records);
                if self.search_save_error.is_none() {
                    self.chrome_crawler_input.clear();
                    self.status_message = format!(
                        "Imported {count} browser records; duplicates merged. Review source excerpts in Contacts before use."
                    );
                    self.add_log("SUCCESS", &self.status_message.clone());
                }
            }
            Err(error) => {
                self.status_message = error.clone();
                self.add_log("ERROR", &error);
            }
        }
    }

    fn run_2gis_import(&mut self) {
        let picked = rfd::FileDialog::new()
            .set_title("Select a 2GIS export or CSV/JSON contact file")
            .add_filter("Contact files (CSV, JSON)", &["csv", "json"])
            .pick_file();

        let target_path = match picked {
            Some(p) => p,
            None => {
                if std::path::Path::new("parser_2gis_new_repo/2gis.csv").exists() {
                    PathBuf::from("parser_2gis_new_repo/2gis.csv")
                } else if std::path::Path::new("2gis.csv").exists() {
                    PathBuf::from("2gis.csv")
                } else if std::path::Path::new("2gis.json").exists() {
                    PathBuf::from("2gis.json")
                } else {
                    self.add_log("INFO", "Import cancelled.");
                    return;
                }
            }
        };

        self.add_log(
            "INFO",
            &format!("Import: reading and parsing {:?}...", target_path),
        );
        match import_external_contacts_file(&target_path) {
            Ok(imported) => {
                let mut added = 0;
                for rec in imported {
                    let exists = if !rec.email.is_empty() {
                        self.contacts.iter().any(|c| {
                            !c.email.is_empty()
                                && c.email.to_lowercase() == rec.email.to_lowercase()
                        })
                    } else {
                        self.contacts.iter().any(|c| {
                            (!c.phone.is_empty() && c.phone == rec.phone)
                                || (!c.company.is_empty()
                                    && c.company == rec.company
                                    && c.city == rec.city)
                        })
                    };
                    if !exists {
                        self.contacts.push(rec);
                        added += 1;
                    }
                }
                let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
                let fname = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");
                self.add_log(
                    "SUCCESS",
                    &format!(
                        "Import complete. Added {} new contacts from {}.",
                        added, fname
                    ),
                );
                self.status_message = format!(
                    "Import successful: +{} contacts. Total: {}",
                    added,
                    self.contacts.len()
                );
            }
            Err(e) => {
                self.add_log("ERROR", &format!("Import error: {}", e));
            }
        }
    }

    fn apply_persona(&mut self, persona_key: &str) {
        let profiles_path = if std::path::Path::new("profiles.json").exists() {
            PathBuf::from("profiles.json")
        } else if std::path::Path::new("../profiles.json").exists() {
            PathBuf::from("../profiles.json")
        } else {
            PathBuf::from("profiles.json")
        };
        if let Ok(content) = std::fs::read_to_string(&profiles_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(p) = val.get("personas").and_then(|ps| ps.get(persona_key)) {
                    let sender_name = p.get("sender_name").and_then(|v| v.as_str()).unwrap_or("");
                    let reply_to = p.get("email").and_then(|v| v.as_str()).unwrap_or("");
                    self.smtp.sender_name = sender_name.to_string();
                    self.smtp.reply_to = reply_to.to_string();
                    let tmpl_val = p.get("outreach_template");
                    self.template = EmailTemplate {
                        template_name: tmpl_val
                            .and_then(|t| t.get("template_name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        subject: tmpl_val
                            .and_then(|t| t.get("subject"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        body: tmpl_val
                            .and_then(|t| t.get("body"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        follow_up_1: None,
                        follow_up_2: None,
                    };
                    let _ = save_app_settings(&self.smtp, &self.template, &self.settings_path);
                    self.add_log(
                        "INFO",
                        &format!(
                            "Active profile changed to '{}' ({})",
                            persona_key, sender_name
                        ),
                    );
                }
            }
        }
    }

    fn poll_async_events(&mut self) {
        // Collect search events without holding borrow
        let search_msgs: Vec<_> = if let Some(ref rx) = self.search_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            Vec::new()
        };

        let mut search_finished = false;
        for msg in search_msgs {
            match msg {
                SearchProgress::Log(level, text) => {
                    self.add_log(&level, &text);
                }
                SearchProgress::Collected(new_records) => {
                    self.search_collected_count = new_records.len();
                    if self.search_save_error.is_none() {
                        self.save_collected_contacts(new_records);
                    }
                }
                SearchProgress::Done(new_records) => {
                    self.search_collected_count = new_records.len();
                    if self.search_save_error.is_none() {
                        self.save_collected_contacts(new_records);
                    }
                    if self.search_save_error.is_none() {
                        self.status_message = if self.automatic_business_collection {
                            format!(
                                "Collected {}/{} companies; saved to database (+{} new). {}",
                                self.search_collected_count,
                                self.filter.max_items,
                                self.contacts
                                    .len()
                                    .saturating_sub(self.search_initial_count),
                                if self.search_collected_count < self.filter.max_items {
                                    "Partial result: collection stopped or sources exhausted. Progress retained."
                                } else {
                                    "Complete."
                                }
                            )
                        } else {
                            format!(
                                "Collection finished: {} database contacts, +{} new. Saved automatically.",
                                self.contacts.len(),
                                self.contacts
                                    .len()
                                    .saturating_sub(self.search_initial_count)
                            )
                        };
                    }
                    search_finished = true;
                }
                SearchProgress::Error(err) => {
                    self.add_log("ERROR", &format!("Collection failed: {}", err));
                    self.status_message = format!("Error: {}", err);
                    search_finished = true;
                }
            }
        }
        if search_finished {
            self.is_searching = false;
            self.search_rx = None;
        }

        if let Some(ref rx) = self.audit_rx {
            if let Ok(res) = rx.try_recv() {
                self.sender_audit_result = Some(res);
                self.is_auditing_sender = false;
                self.audit_rx = None;
                self.add_log("SUCCESS", "Sender domain audit complete (SPF/DMARC/DNSBL).");
            }
        }

        // Collect mailer events without holding borrow
        let mailer_msgs: Vec<_> = if let Some(ref rx) = self.mailer_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            Vec::new()
        };

        let mut mailer_finished = false;
        for msg in mailer_msgs {
            match msg {
                MailerProgress::Log(level, text) => {
                    self.add_log(&level, &text);
                }
                MailerProgress::ItemSent {
                    id,
                    email,
                    success,
                    error_msg,
                    current,
                    total,
                    message_id,
                } => {
                    if success {
                        self.total_sent += 1;
                    } else {
                        self.total_failed += 1;
                    }
                    if let Some(item) = self
                        .contacts
                        .iter_mut()
                        .find(|c| c.id == id || c.email == email)
                    {
                        if success {
                            item.status = SendStatus::Sent;
                            item.follow_up_count += 1;
                            item.last_sent_at =
                                Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                            if let Some(mid) = message_id {
                                item.last_message_id = Some(mid);
                            }
                            self.smtp.sent_today += 1;
                            let _ =
                                save_app_settings(&self.smtp, &self.template, &self.settings_path);
                        } else {
                            let error = error_msg.unwrap_or_else(|| "Unknown error".to_string());
                            let bounce = search::bounce_classifier::classify_smtp_bounce(&error);
                            item.status = SendStatus::Failed(error);
                            if bounce.should_suppress {
                                item.selected = false;
                                self.suppression_list.add_email(&email);
                                let _ = save_suppression_list(
                                    &self.suppression_list,
                                    &self.suppression_path,
                                );
                                self.add_log(
                                    "WARN",
                                    &format!("{email}: mailbox does not exist, added to suppression list."),
                                );
                            } else if bounce.bounce_type
                                == search::bounce_classifier::BounceType::SpamBlock
                            {
                                self.add_log(
                                    "WARN",
                                    &format!("{email}: rejected as spam by the recipient server. Stop the campaign and check sender reputation before sending more."),
                                );
                            }
                        }
                    }
                    self.status_message = format!("Mailing: sent {} of {}...", current, total);
                }
                MailerProgress::Done { sent, failed } => {
                    self.add_log(
                        "SUCCESS",
                        &format!("Mailing complete. Sent: {}, failed: {}", sent, failed),
                    );
                    self.status_message =
                        format!("Mailing finished: {} sent, {} failed.", sent, failed);
                    let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
                    mailer_finished = true;
                }
                MailerProgress::TestDone(result) => {
                    let (level, message) = match result {
                        Ok(()) => ("SUCCESS", "Test email accepted by the mail server. Check the recipient mailbox for delivery and the displayed sender.".to_string()),
                        Err(error) => ("ERROR", format!("Test email failed: {error}")),
                    };
                    self.status_message = message.clone();
                    self.add_log(level, &message);
                    mailer_finished = true;
                }
                MailerProgress::Error(err) => {
                    self.add_log("ERROR", &format!("Mailing failed: {}", err));
                    self.status_message = format!("Mailing error: {}", err);
                    mailer_finished = true;
                }
            }
        }
        if mailer_finished {
            self.is_mailing = false;
            self.mailer_rx = None;
        }
    }
}

impl eframe::App for AppState {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_async_events();
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_workspace(ui);
        #[cfg(debug_assertions)]
        if let Ok(path) = std::env::var("POISK_UI_CAPTURE") {
            let review = std::env::var("POISK_UI_REVIEW").unwrap_or_default();
            let ready = if review.contains("places") || review.contains("countries") {
                self.place_suggestions.capture_ready()
            } else if review.contains("topics") {
                self.topic_suggestions.capture_ready()
            } else {
                true
            };
            let capture_id = egui::Id::new("ui_capture_requested");
            if ui.ctx().cumulative_frame_nr() >= 5
                && ready
                && !ui
                    .ctx()
                    .data(|data| data.get_temp::<bool>(capture_id).unwrap_or(false))
            {
                ui.ctx().data_mut(|data| data.insert_temp(capture_id, true));
                ui.ctx()
                    .send_viewport_cmd(
                        egui::ViewportCommand::Screenshot(egui::UserData::default()),
                    );
            }
            let screenshot = ui.input(|i| {
                i.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = screenshot {
                let mut data =
                    format!("P6\n{} {}\n255\n", image.size[0], image.size[1]).into_bytes();
                for pixel in &image.pixels {
                    data.extend_from_slice(&pixel.to_array()[..3]);
                }
                std::fs::write(path, data).expect("Could not save UI review screenshot");
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

fn draw_brand_logo(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 6.0, Color32::from_rgb(20, 20, 24));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, BORDER_COLOR),
        egui::StrokeKind::Inside,
    );

    let center = rect.center();
    let zinc_faint = Color32::from_rgb(113, 113, 122);
    let zinc_bright = Color32::from_rgb(228, 228, 231);

    // Radar scan aperture ring
    painter.circle_stroke(center, 5.5, Stroke::new(1.15, zinc_faint));

    // Targeting ticks (West, South, East)
    painter.line_segment(
        [
            Pos2::new(center.x - 7.0, center.y),
            Pos2::new(center.x - 4.2, center.y),
        ],
        Stroke::new(1.15, zinc_faint),
    );
    painter.line_segment(
        [
            Pos2::new(center.x, center.y + 4.2),
            Pos2::new(center.x, center.y + 7.0),
        ],
        Stroke::new(1.15, zinc_faint),
    );
    painter.line_segment(
        [
            Pos2::new(center.x + 4.2, center.y),
            Pos2::new(center.x + 7.0, center.y),
        ],
        Stroke::new(1.15, zinc_faint),
    );

    // Active outreach beam to North-East (45 degrees)
    let beam_tip = Pos2::new(center.x + 4.8, center.y - 4.8);
    painter.line_segment([center, beam_tip], Stroke::new(1.4, Color32::WHITE));

    // Discovered contact target node (satellite spark)
    painter.circle_filled(beam_tip, 1.6, Color32::WHITE);

    // Center origin radar core
    painter.circle_filled(center, 1.8, zinc_bright);
}

fn draw_tab_icon(painter: &egui::Painter, tab: ActiveTab, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.25, color);
    match tab {
        ActiveTab::Search => {
            let lens_center = Pos2::new(c.x - 1.5, c.y - 1.5);
            painter.circle_stroke(lens_center, 4.0, stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x + 1.5, c.y + 1.5),
                    Pos2::new(c.x + 5.2, c.y + 5.2),
                ],
                Stroke::new(1.5, color),
            );
            painter.circle_filled(lens_center, 1.1, color);
        }
        ActiveTab::Contacts => {
            let head_center = Pos2::new(c.x, c.y - 3.2);
            painter.circle_stroke(head_center, 2.5, stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x - 4.5, c.y + 4.5),
                    Pos2::new(c.x - 2.5, c.y + 1.8),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, c.y + 1.8),
                    Pos2::new(c.x + 2.5, c.y + 1.8),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 2.5, c.y + 1.8),
                    Pos2::new(c.x + 5.0, c.y + 4.5),
                ],
                stroke,
            );
        }
        ActiveTab::Template => {
            let doc_rect = egui::Rect::from_center_size(c, Vec2::new(9.5, 12.0));
            painter.rect_stroke(doc_rect, 1.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, c.y - 2.0),
                    Pos2::new(c.x + 2.5, c.y - 2.0),
                ],
                Stroke::new(1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 2.5, c.y + 0.8),
                    Pos2::new(c.x + 1.2, c.y + 0.8),
                ],
                Stroke::new(1.0, color),
            );
        }
        ActiveTab::Smtp => {
            let env_rect = egui::Rect::from_center_size(c, Vec2::new(12.0, 9.0));
            painter.rect_stroke(env_rect, 1.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(env_rect.left() + 1.0, env_rect.top() + 1.0),
                    Pos2::new(c.x, c.y + 0.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(env_rect.right() - 1.0, env_rect.top() + 1.0),
                    Pos2::new(c.x, c.y + 0.5),
                ],
                stroke,
            );
        }
        ActiveTab::Logs => {
            painter.line_segment(
                [
                    Pos2::new(c.x - 4.5, c.y - 3.5),
                    Pos2::new(c.x - 1.5, c.y - 0.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 1.5, c.y - 0.5),
                    Pos2::new(c.x - 4.5, c.y + 2.5),
                ],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y + 3.0), Pos2::new(c.x + 4.5, c.y + 3.0)],
                Stroke::new(1.3, color),
            );
        }
    }
}

impl AppState {
    fn render_workspace(&mut self, ui: &mut egui::Ui) {
        for (key, tab) in [
            (egui::Key::Num1, ActiveTab::Search),
            (egui::Key::Num2, ActiveTab::Contacts),
            (egui::Key::Num3, ActiveTab::Template),
            (egui::Key::Num4, ActiveTab::Smtp),
            (egui::Key::Num5, ActiveTab::Logs),
        ] {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                self.active_tab = tab;
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            self.show_command_palette = !self.show_command_palette;
            self.command_palette_query.clear();
            self.command_palette_selected = 0;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
            self.show_shortcuts_modal = !self.show_shortcuts_modal;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::D)) {
            self.compact_density = !self.compact_density;
            let mode = if self.compact_density {
                "Compact (+40% rows visible)"
            } else {
                "Relaxed"
            };
            self.show_toast(format!("Table density: {mode}"), true);
        }
        if self.active_tab == ActiveTab::Contacts
            && !ui.ctx().egui_wants_keyboard_input()
            && !self.show_command_palette
            && !self.show_shortcuts_modal
        {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::X)) {
                let toast_msg = if let Some(ref sel_id) = self.selected_contact_id {
                    if let Some(c) = self.contacts.iter_mut().find(|c| &c.id == sel_id) {
                        c.selected = !c.selected;
                        let st = if c.selected { "selected" } else { "deselected" };
                        Some(format!("Lead {st}: {}", c.company))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(msg) = toast_msg {
                    self.show_toast(msg, true);
                }
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                if !self.contacts.is_empty() {
                    let cur_idx = self
                        .selected_contact_id
                        .as_ref()
                        .and_then(|id| self.contacts.iter().position(|c| &c.id == id))
                        .unwrap_or(0);
                    let next_idx = (cur_idx + 1).min(self.contacts.len() - 1);
                    self.selected_contact_id = Some(self.contacts[next_idx].id.clone());
                }
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                if !self.contacts.is_empty() {
                    let cur_idx = self
                        .selected_contact_id
                        .as_ref()
                        .and_then(|id| self.contacts.iter().position(|c| &c.id == id))
                        .unwrap_or(0);
                    let prev_idx = cur_idx.saturating_sub(1);
                    self.selected_contact_id = Some(self.contacts[prev_idx].id.clone());
                }
            }
        }
        egui::Panel::bottom("status_bar")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(13, 13, 16))
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .inner_margin(egui::Margin::symmetric(14, 5)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 14.0;
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new("Ctrl K")
                                        .monospace()
                                        .small()
                                        .color(TEXT_FAINT),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text("Open Command Palette (Ctrl+K)")
                            .clicked()
                        {
                            self.show_command_palette = true;
                            self.command_palette_query.clear();
                            self.command_palette_selected = 0;
                        }
                        ui.label(RichText::new("·").small().color(TEXT_FAINT));
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new("? Shortcuts")
                                        .monospace()
                                        .small()
                                        .color(TEXT_FAINT),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text("View keyboard shortcuts (F1)")
                            .clicked()
                        {
                            self.show_shortcuts_modal = true;
                        }
                        ui.label(RichText::new("·").small().color(TEXT_FAINT));
                        ui.label(
                            RichText::new("Ctrl 1..5")
                                .monospace()
                                .small()
                                .color(TEXT_FAINT),
                        );
                        ui.label(RichText::new("·").small().color(TEXT_FAINT));
                        ui.label(
                            RichText::new(format!("{} leads", self.contacts.len()))
                                .small()
                                .color(TEXT_MUTED),
                        );
                    });
                    if self.is_searching {
                        ui.label(
                            RichText::new("● Crawling")
                                .small()
                                .color(ACCENT_AMBER)
                                .strong(),
                        );
                    } else if self.is_mailing {
                        ui.label(
                            RichText::new("● Sending")
                                .small()
                                .color(ACCENT_CYAN)
                                .strong(),
                        );
                    } else if self.is_auditing_sender {
                        ui.label(
                            RichText::new("● Auditing")
                                .small()
                                .color(ACCENT_BLUE)
                                .strong(),
                        );
                    } else {
                        ui.label(
                            RichText::new("● Ready")
                                .small()
                                .color(Color32::from_rgb(74, 222, 128)),
                        );
                    }
                    ui.label(RichText::new("·").small().color(TEXT_FAINT));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&self.status_message)
                                .small()
                                .color(TEXT_MUTED),
                        )
                        .truncate(),
                    )
                    .on_hover_text(&self.status_message);
                });
            });
        if ui.available_width() < COMPACT_WIDTH {
            egui::Panel::top("compact_navigation")
                .frame(
                    egui::Frame::new()
                        .fill(CARD_BG)
                        .inner_margin(egui::Margin::symmetric(16, 8)),
                )
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        draw_brand_logo(ui.painter(), rect);
                        ui.add_space(4.0);
                        ui.label(RichText::new("PoiskContact").strong().color(Color32::WHITE));
                        egui::ComboBox::from_id_salt("workspace_section")
                            .selected_text(
                                WORKSPACE_TABS
                                    .iter()
                                    .find(|(tab, _)| *tab == self.active_tab)
                                    .unwrap()
                                    .1,
                            )
                            .show_ui(ui, |ui| {
                                for (tab, label) in WORKSPACE_TABS {
                                    ui.selectable_value(&mut self.active_tab, tab, label);
                                }
                            });
                        self.persona_menu(ui);
                    });
                });
        } else {
            egui::Panel::left("navigation")
                .resizable(false)
                .default_size(NAV_WIDTH)
                .min_size(NAV_WIDTH)
                .max_size(NAV_WIDTH)
                .frame(
                    egui::Frame::new()
                        .fill(SIDEBAR_BG)
                        .stroke(Stroke::new(1.0, BORDER_COLOR))
                        .inner_margin(16.0),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                            draw_brand_logo(ui.painter(), rect);
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("PoiskContact")
                                    .strong()
                                    .size(16.0)
                                    .color(Color32::WHITE),
                            );
                        });
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new("WORKSPACE")
                                .small()
                                .color(TEXT_FAINT)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        self.navigation(ui);
                        ui.add_space(18.0);
                        ui.separator();
                        ui.add_space(14.0);
                        self.render_sender_profile_card(ui);
                        ui.add_space(12.0);
                        self.render_database_metrics_card(ui);
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            render_kbd(ui, "Ctrl");
                            ui.add_space(2.0);
                            render_kbd(ui, "1…5");
                            ui.add_space(5.0);
                            ui.label(RichText::new("switch tabs").small().color(TEXT_FAINT));
                        });
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_COLOR).inner_margin(
                if ui.available_width() < COMPACT_WIDTH {
                    16.0
                } else {
                    PAGE_MARGIN
                },
            ))
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(1120.0));
                match self.active_tab {
                    ActiveTab::Search => self.render_search_tab(ui),
                    ActiveTab::Contacts => {
                        egui::ScrollArea::vertical()
                            .id_salt("contacts_workspace")
                            .show(ui, |ui| self.render_contacts_tab(ui));
                    }
                    ActiveTab::Template => self.render_template_tab(ui),
                    ActiveTab::Smtp => self.render_smtp_tab(ui),
                    ActiveTab::Logs => self.render_logs_tab(ui),
                }
            });
        self.render_toast(ui.ctx());
        self.render_shortcuts_modal(ui.ctx());
        self.render_command_palette(ui.ctx());
        self.deletion_dialog(ui.ctx());
        self.render_quick_outreach_modal(ui.ctx());
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
        for (tab, label) in WORKSPACE_TABS {
            let active = self.active_tab == tab;
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), CONTROL_HEIGHT),
                egui::Sense::click(),
            );
            let is_hovered = response.hovered();
            let bg_fill = if active {
                Color32::from_rgb(26, 26, 30)
            } else if is_hovered {
                Color32::from_rgb(18, 18, 22)
            } else {
                Color32::TRANSPARENT
            };
            let stroke = if active {
                Stroke::new(1.0, BORDER_COLOR)
            } else {
                Stroke::NONE
            };

            ui.painter().rect(
                rect,
                egui::CornerRadius::same(6),
                bg_fill,
                stroke,
                egui::StrokeKind::Inside,
            );

            if active {
                ui.painter().vline(
                    rect.left() + 1.5,
                    rect.y_range().shrink(7.0),
                    Stroke::new(2.0, Color32::WHITE),
                );
            }

            let icon_center = Pos2::new(rect.left() + 16.0, rect.center().y);
            let icon_color = if active {
                Color32::WHITE
            } else if is_hovered {
                Color32::from_rgb(228, 228, 231)
            } else {
                TEXT_MUTED
            };
            draw_tab_icon(ui.painter(), tab, icon_center, icon_color);

            let text_pos = Pos2::new(rect.left() + 32.0, rect.center().y);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                if active { Color32::WHITE } else { TEXT_MUTED },
            );

            if response.clicked() {
                self.active_tab = tab;
            }
        }
    }

    fn render_sender_profile_card(&mut self, ui: &mut egui::Ui) {
        let role_desc = self.template.template_name.clone();
        let display_name = if self.smtp.sender_name.trim().is_empty() {
            "Configure your sender".to_string()
        } else {
            self.smtp.sender_name.clone()
        };
        let email_str = if !self.smtp.reply_to.trim().is_empty() {
            self.smtp.reply_to.clone()
        } else if !self.smtp.from_email.trim().is_empty() {
            self.smtp.from_email.clone()
        } else {
            "No email configured".to_string()
        };

        egui::Frame::new()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0, BORDER_COLOR))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (indicator_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(6.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        indicator_rect.center(),
                        2.5,
                        Color32::from_rgb(52, 211, 153),
                    );
                    ui.label(RichText::new("SENDER").small().color(TEXT_FAINT).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.persona_menu(ui);
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(display_name)
                        .strong()
                        .color(Color32::WHITE)
                        .size(13.0),
                );
                ui.label(
                    RichText::new(role_desc)
                        .small()
                        .color(Color32::from_rgb(161, 161, 170)),
                );
                ui.add(
                    egui::Label::new(RichText::new(&email_str).small().color(TEXT_FAINT))
                        .truncate(),
                )
                .on_hover_text(&email_str);
            });
    }

    fn render_database_metrics_card(&self, ui: &mut egui::Ui) {
        let total = self.contacts.len();
        let with_mx = self.contacts.iter().filter(|c| c.mx_valid).count();
        let selected = self.contacts.iter().filter(|c| c.selected).count();
        let mx_pct = if total > 0 {
            (with_mx * 100) / total
        } else {
            0
        };

        egui::Frame::new()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0, BORDER_COLOR))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("DATABASE POOL")
                            .small()
                            .color(TEXT_FAINT)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if total > 0 {
                            ui.label(
                                RichText::new(format!("{mx_pct}% MX"))
                                    .small()
                                    .color(Color32::from_rgb(161, 161, 170)),
                            );
                        }
                    });
                });
                ui.add_space(6.0);
                for (icon, label, count) in [
                    ("●", "Contacts", total),
                    ("✓", "With MX", with_mx),
                    ("■", "Selected", selected),
                ] {
                    ui.horizontal(|ui| {
                        let dot_color = match label {
                            "With MX" => Color32::from_rgb(52, 211, 153),
                            "Selected" if count > 0 => Color32::from_rgb(96, 165, 250),
                            _ => TEXT_MUTED,
                        };
                        ui.label(RichText::new(icon).small().color(dot_color));
                        ui.label(RichText::new(label).small().color(TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(count.to_string())
                                    .small()
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });
                    });
                }
            });
    }

    fn persona_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Profile presets", |ui| {
            ui.label("Configure profiles.json to use these presets.");
            if ui.button("Creative (photo and video)").clicked() {
                self.apply_persona("creative");
                ui.close();
            }
            if ui.button("Security (AppSec)").clicked() {
                self.apply_persona("cybersec");
                ui.close();
            }
        });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File and export", |ui| {
            if ui.button("Save JSON database").clicked() {
                match save_contacts_to_json(&self.contacts, &self.default_json_path) {
                    Ok(()) => {
                        self.status_message = "Contact database saved.".into();
                        self.show_toast("Contact database saved", true);
                    }
                    Err(e) => {
                        self.status_message = e.clone();
                        self.add_log("ERROR", &e);
                        self.show_toast("Failed to save database", false);
                    }
                }
                ui.close();
            }
            if ui
                .add_enabled(
                    !self.is_searching && !self.is_mailing,
                    egui::Button::new("Load JSON database"),
                )
                .clicked()
            {
                match load_contacts_from_json(&self.default_json_path) {
                    Ok(loaded) => {
                        self.contacts = loaded;
                        self.selected_contact_id = None;
                        self.status_message = format!("Loaded {} contacts.", self.contacts.len());
                        self.show_toast(format!("Loaded {} contacts", self.contacts.len()), true);
                    }
                    Err(e) => {
                        self.status_message = e.clone();
                        self.add_log("ERROR", &e);
                        self.show_toast("Failed to load database", false);
                    }
                }
                ui.close();
            }
            ui.separator();
            for (label, filename) in [
                ("CSV / Excel", "contacts.csv"),
                ("vCard (.vcf)", "contacts.vcf"),
                ("AmoCRM CSV", "contacts_amocrm.csv"),
                ("Bitrix24 CSV", "contacts_bitrix24.csv"),
                ("Instantly CSV", "contacts_instantly.csv"),
                ("Lemlist CSV", "contacts_lemlist.csv"),
            ] {
                if ui.button(label).clicked() {
                    let path = self.default_json_path.with_file_name(filename);
                    let result = match filename {
                        "contacts.vcf" => export_contacts_to_vcf(&self.contacts, &path),
                        "contacts_amocrm.csv" => {
                            export_contacts_to_amocrm_csv(&self.contacts, &path)
                        }
                        "contacts_bitrix24.csv" => {
                            export_contacts_to_bitrix24_csv(&self.contacts, &path)
                        }
                        "contacts_instantly.csv" => {
                            export_contacts_to_instantly_csv(&self.contacts, &path)
                        }
                        "contacts_lemlist.csv" => {
                            export_contacts_to_lemlist_csv(&self.contacts, &path)
                        }
                        _ => export_contacts_to_csv(&self.contacts, &self.default_csv_path),
                    };
                    match result {
                        Ok(()) => {
                            self.status_message = format!("Export saved: {}", path.display());
                            self.show_toast(format!("Export saved: {filename}"), true);
                        }
                        Err(e) => {
                            self.status_message = e.clone();
                            self.add_log("ERROR", &e);
                            self.show_toast("Export failed", false);
                        }
                    }
                    ui.close();
                }
            }
        });
    }

    fn mailing_actions(&mut self, ui: &mut egui::Ui) {
        let selected = self
            .contacts
            .iter()
            .filter(|c| {
                c.selected
                    && c.status == SendStatus::Pending
                    && !self.suppression_list.is_suppressed(&c.email)
            })
            .count();
        ui.horizontal_wrapped(|ui| {
            let label = if self.is_mailing {
                "Sending emails...".into()
            } else {
                format!("Send emails ({selected})")
            };
            if primary_button(
                ui,
                !self.is_mailing && !self.is_searching && selected > 0,
                label,
            )
            .on_disabled_hover_text("Select contacts and wait for the current operation to finish.")
            .clicked()
            {
                self.start_mailing();
                self.active_tab = ActiveTab::Logs;
            }
            let followable = self
                .contacts
                .iter()
                .filter(|c| {
                    c.selected
                        && c.status == SendStatus::Sent
                        && c.follow_up_count >= 1
                        && c.follow_up_count < 3
                        && !self.suppression_list.is_suppressed(&c.email)
                })
                .count();
            if ui
                .add_enabled(
                    !self.is_mailing && !self.is_searching && followable > 0,
                    egui::Button::new(format!("Follow-up ({followable})")),
                )
                .on_hover_text("Send a follow-up to eligible selected recipients.")
                .on_disabled_hover_text("No selected contacts are eligible for follow-up.")
                .clicked()
            {
                self.start_followup_mailing();
                self.active_tab = ActiveTab::Logs;
            }
            if ui.button("Check template").clicked() {
                self.active_tab = ActiveTab::Template;
            }
        });
    }

    fn deletion_dialog(&mut self, ctx: &egui::Context) {
        if !self.confirm_clear && self.pending_delete.is_none() {
            return;
        }
        let mut confirm = false;
        let response = egui::Modal::new(egui::Id::new("delete_contacts")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.heading(if self.confirm_clear {
                "Clear the contact database?"
            } else {
                "Delete this contact?"
            });
            ui.label(
                "Records will be removed from the database and JSON file. This cannot be undone.",
            );
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                confirm = ui
                    .add(egui::Button::new(
                        RichText::new(if self.confirm_clear {
                            "Delete all contacts"
                        } else {
                            "Delete contact"
                        })
                        .color(ui.visuals().error_fg_color),
                    ))
                    .clicked();
                if ui.button("Cancel").clicked() {
                    self.confirm_clear = false;
                    self.pending_delete = None;
                }
            });
        });
        if confirm {
            let remaining: Vec<_> = self
                .contacts
                .iter()
                .filter(|c| {
                    !self.confirm_clear && self.pending_delete.as_deref() != Some(c.id.as_str())
                })
                .cloned()
                .collect();
            match save_contacts_to_json(&remaining, &self.default_json_path) {
                Ok(()) => {
                    self.contacts = remaining;
                    if self.confirm_clear || self.selected_contact_id == self.pending_delete {
                        self.selected_contact_id = None;
                    }
                    self.status_message = "Contacts deleted. Database saved.".into();
                    self.confirm_clear = false;
                    self.pending_delete = None;
                }
                Err(e) => {
                    self.status_message = format!("Could not save the database: {e}");
                    self.add_log("ERROR", &e);
                }
            }
        } else if response.should_close() {
            self.confirm_clear = false;
            self.pending_delete = None;
        }
    }

    fn render_quick_outreach_modal(&mut self, ctx: &egui::Context) {
        let Some(contact_id) = self.quick_outreach_contact_id.clone() else {
            return;
        };
        let Some(contact) = self.contacts.iter().find(|c| c.id == contact_id).cloned() else {
            self.quick_outreach_contact_id = None;
            return;
        };

        let mut should_close = false;
        let mut send_requested = false;

        egui::Modal::new(egui::Id::new("quick_outreach_modal")).show(ctx, |ui| {
            ui.set_max_width(580.0);
            ui.horizontal(|ui| {
                ui.heading("Direct Outreach");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        should_close = true;
                    }
                    render_kbd(ui, "Esc");
                });
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "Send personalized email directly to {} ({})",
                    contact.name, contact.company
                ))
                .weak()
                .small(),
            );
            ui.add_space(8.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Recipient:").strong());
                        ui.label(RichText::new(&contact.email).strong().color(Color32::WHITE));
                        if contact.mx_valid {
                            render_status_pill(
                                ui,
                                "MX Active",
                                ACCENT_TEAL,
                                Color32::from_rgb(10, 32, 24),
                                Color32::from_rgb(18, 64, 46),
                            );
                        } else {
                            render_status_pill(
                                ui,
                                "MX Unverified",
                                ACCENT_AMBER,
                                Color32::from_rgb(36, 26, 14),
                                Color32::from_rgb(64, 44, 20),
                            );
                        }
                        if contact.rcpt_verified {
                            render_status_pill(
                                ui,
                                "RCPT 250 OK",
                                ACCENT_CYAN,
                                Color32::from_rgb(12, 28, 38),
                                Color32::from_rgb(20, 52, 70),
                            );
                        }
                        if self.suppression_list.is_suppressed(&contact.email) {
                            render_status_pill(
                                ui,
                                "Suppressed",
                                ACCENT_ROSE,
                                Color32::from_rgb(36, 14, 18),
                                Color32::from_rgb(64, 22, 28),
                            );
                        }
                        let mode = if self.smtp.direct_mx {
                            "Direct MX"
                        } else {
                            "SMTP Relay"
                        };
                        ui.label(
                            RichText::new(format!("via {mode}"))
                                .small()
                                .color(TEXT_FAINT),
                        );
                    });
                });

            ui.add_space(10.0);
            ui.label(RichText::new("Subject line:").strong());
            ui.add(
                egui::TextEdit::singleline(&mut self.quick_outreach_subject)
                    .desired_width(ui.available_width())
                    .hint_text("Subject line..."),
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Personalized message body:").strong());
            ui.add(
                egui::TextEdit::multiline(&mut self.quick_outreach_body)
                    .desired_width(ui.available_width())
                    .desired_rows(9)
                    .hint_text("Message body..."),
            );

            if contact.follow_up_count > 0 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Follow-up #{} · Will thread into previous message conversation",
                        contact.follow_up_count + 1
                    ))
                    .small()
                    .color(ACCENT_BLUE),
                );
            }

            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui| {
                let can_send = !self.is_mailing
                    && !contact.email.trim().is_empty()
                    && !self.suppression_list.is_suppressed(&contact.email)
                    && !self.quick_outreach_subject.trim().is_empty()
                    && !self.quick_outreach_body.trim().is_empty();

                let send_btn_text = if self.is_mailing {
                    "Sending...".to_string()
                } else {
                    format!("Send Direct Email to {}", contact.email)
                };

                if ui
                    .add_enabled(
                        can_send,
                        egui::Button::new(
                            RichText::new(send_btn_text).color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(37, 99, 235)),
                    )
                    .clicked()
                {
                    send_requested = true;
                }

                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

        if send_requested {
            let subj = self.quick_outreach_subject.clone();
            let bdy = self.quick_outreach_body.clone();
            self.send_direct_contact_email(contact_id, subj, bdy);
            self.quick_outreach_contact_id = None;
        } else if should_close {
            self.quick_outreach_contact_id = None;
        }
    }

    fn render_search_criteria(&mut self, ui: &mut egui::Ui, _min_height: f32) {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Search criteria")
                        .strong()
                        .size(15.0)
                        .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Reset defaults").small().color(TEXT_MUTED),
                            )
                            .fill(Color32::TRANSPARENT),
                        )
                        .on_hover_text(
                            "Restore default search location, niche and collection settings",
                        )
                        .clicked()
                    {
                        self.filter = SearchFilter::default();
                        self.all_industries = false;
                        self.automatic_business_collection = true;
                        self.show_toast("Search criteria reset to defaults", false);
                    }
                });
            });
            ui.add_space(10.0);

            let has_active_filters = !self.filter.city.trim().is_empty()
                || (!self.filter.stack.trim().is_empty()
                    && (!self.automatic_business_collection || !self.all_industries))
                || self.filter.role_filter != "All";

            if has_active_filters {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Active:").small().color(TEXT_FAINT));
                    if !self.filter.city.trim().is_empty() {
                        let label = format!("Location: {} ×", self.filter.city.trim());
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(label)
                                        .small()
                                        .color(Color32::from_rgb(147, 197, 253)),
                                )
                                .fill(Color32::from_rgb(15, 23, 42))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(30, 58, 138))),
                            )
                            .on_hover_text("Click to clear location filter")
                            .clicked()
                        {
                            self.filter.city.clear();
                            self.show_toast("Cleared location filter", true);
                        }
                    }
                    if !self.filter.stack.trim().is_empty()
                        && (!self.automatic_business_collection || !self.all_industries)
                    {
                        let truncated = if self.filter.stack.chars().count() > 22 {
                            format!(
                                "{}...",
                                self.filter.stack.chars().take(20).collect::<String>()
                            )
                        } else {
                            self.filter.stack.clone()
                        };
                        let label = format!("Niche: {} ×", truncated);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(label)
                                        .small()
                                        .color(Color32::from_rgb(167, 139, 250)),
                                )
                                .fill(Color32::from_rgb(26, 16, 48))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(76, 29, 149))),
                            )
                            .on_hover_text("Click to clear industry filter")
                            .clicked()
                        {
                            self.filter.stack.clear();
                            self.show_toast("Cleared industry filter", true);
                        }
                    }
                    if self.filter.role_filter != "All" {
                        let role_name = match self.filter.role_filter.as_str() {
                            "decision_makers" => "Decision Makers",
                            "hr" => "HR & Recruiters",
                            other => other,
                        };
                        let label = format!("ICP: {} ×", role_name);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(label)
                                        .small()
                                        .color(Color32::from_rgb(251, 191, 36)),
                                )
                                .fill(Color32::from_rgb(45, 26, 12))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(146, 64, 14))),
                            )
                            .on_hover_text("Click to reset ICP to all contacts")
                            .clicked()
                        {
                            self.filter.role_filter = "All".to_string();
                            self.show_toast("Reset ICP to all contacts", true);
                        }
                    }
                });
                ui.add_space(8.0);
            }
            ui.label(
                RichText::new("Target Location")
                    .strong()
                    .color(Color32::WHITE),
            );
            self.place_suggestions.ui(
                ui,
                &mut self.filter.city,
                "City, region or country",
                autocomplete::Source::Places,
                &self.tokio_rt,
            );
            ui.label(
                RichText::new("Leave empty for a worldwide search.")
                    .small()
                    .color(TEXT_FAINT),
            );
            ui.add_space(14.0);
            ui.label(
                RichText::new("Collection mode")
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.add_enabled_ui(!self.is_searching, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.radio_value(
                        &mut self.automatic_business_collection,
                        true,
                        "Business websites",
                    );
                    ui.radio_value(
                        &mut self.automatic_business_collection,
                        false,
                        "Selected sources",
                    );
                });
                if self.automatic_business_collection {
                    ui.checkbox(&mut self.all_industries, "Search across industries");
                }
            });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Niche presets:").small().color(TEXT_FAINT));
                for (label, niche) in [
                    ("IT & Dev", "it компании разработка софта"),
                    ("HoReCa", "рестораны кафе банкетные залы"),
                    ("Factories", "производственные предприятия заводы"),
                    ("Agency", "digital агентство реклама"),
                ] {
                    let active = self.filter.stack == niche;
                    if render_filter_chip(ui, active, label, &format!("Fill niche: {niche}"))
                        .clicked()
                    {
                        self.filter.stack = niche.to_string();
                        self.all_industries = false;
                        self.show_toast(format!("Applied niche: {label}"), true);
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Target ICP:").small().color(TEXT_FAINT));
                for (label, role_query, role_kind) in [
                    (
                        "CEO / Founder",
                        "генеральный директор основатель",
                        "decision_makers",
                    ),
                    (
                        "CTO / Tech Lead",
                        "технический директор cto тимлид",
                        "decision_makers",
                    ),
                    (
                        "Head of Sales",
                        "коммерческий директор руководитель продаж",
                        "decision_makers",
                    ),
                    ("HRD / Talent", "директор по персоналу hr тим", "hr"),
                ] {
                    let active = self.filter.role_filter == role_kind
                        && self.filter.stack.contains(role_query);
                    if render_filter_chip(ui, active, label, &format!("Target role: {label}"))
                        .clicked()
                    {
                        self.filter.role_filter = role_kind.to_string();
                        if self.filter.stack.is_empty() || self.all_industries {
                            self.filter.stack = role_query.to_string();
                            self.all_industries = false;
                        } else if !self.filter.stack.contains(role_query) {
                            self.filter.stack = format!("{} {}", self.filter.stack, role_query);
                        }
                        self.show_toast(format!("Applied ICP: {label}"), true);
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Saved dorks:").small().color(TEXT_FAINT));
                for (label, query, city, role) in [
                    (
                        "SaaS Founders",
                        "saas b2b cloud platform",
                        "",
                        "decision_makers",
                    ),
                    (
                        "Fintech CTO",
                        "fintech payment banking",
                        "",
                        "decision_makers",
                    ),
                    (
                        "HoReCa Krasnoyarsk",
                        "ресторан кафе бар horeca",
                        "Krasnoyarsk",
                        "decision_makers",
                    ),
                    (
                        "E-com Directors",
                        "интернет-магазин ecommerce ритейл",
                        "",
                        "decision_makers",
                    ),
                ] {
                    let active = self.filter.stack == query;
                    if render_filter_chip(ui, active, label, &format!("Load saved dork: {label}"))
                        .clicked()
                    {
                        self.filter.stack = query.to_string();
                        if !city.is_empty() {
                            self.filter.city = city.to_string();
                        }
                        self.filter.role_filter = role.to_string();
                        self.all_industries = false;
                        self.show_toast(format!("Applied saved dork: {label}"), true);
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("Business or service")
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.add_enabled_ui(
                !self.automatic_business_collection || !self.all_industries,
                |ui| {
                    self.topic_suggestions.ui(
                        ui,
                        &mut self.filter.stack,
                        "Industry, profession or service",
                        autocomplete::Source::Topics,
                        &self.tokio_rt,
                    );
                },
            );
            if self.automatic_business_collection && self.all_industries {
                ui.label(
                    RichText::new("Categories are selected automatically.")
                        .small()
                        .color(TEXT_MUTED),
                );
            }
            ui.add_space(14.0);
            egui::Grid::new("collection_scope")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Contact person").strong());
                    let role = match self.filter.role_filter.as_str() {
                        "decision_makers" | "руковод" => "Decision makers",
                        "hr" => "HR and recruiters",
                        _ => "All contacts",
                    };
                    egui::ComboBox::from_id_salt("search_contact_role")
                        .selected_text(role)
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("All", "All contacts"),
                                ("decision_makers", "Decision makers"),
                                ("hr", "HR and recruiters"),
                            ] {
                                ui.selectable_value(
                                    &mut self.filter.role_filter,
                                    value.into(),
                                    label,
                                );
                            }
                        });
                    ui.end_row();
                    ui.label(
                        RichText::new(if self.automatic_business_collection {
                            "Companies per run"
                        } else {
                            "Contacts per run"
                        })
                        .strong(),
                    );
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [80.0, CONTROL_HEIGHT],
                            egui::DragValue::new(&mut self.filter.max_items).range(1..=usize::MAX),
                        );
                        for preset in [50, 100, 250, 500] {
                            let active = self.filter.max_items == preset;
                            if ui
                                .selectable_label(active, format!("{preset}"))
                                .on_hover_text(format!("Set limit to {preset}"))
                                .clicked()
                            {
                                self.filter.max_items = preset;
                            }
                        }
                    });
                    ui.end_row();
                });
            ui.add_space(10.0);
            let est_domains = (self.filter.max_items as f32 * 0.85).round() as usize;
            let est_emails = (self.filter.max_items as f32 * 0.92).round() as usize;
            let est_mx = 88;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Yield estimate:").small().color(TEXT_FAINT));
                render_status_pill(
                    ui,
                    &format!("~{} domains", est_domains),
                    Color32::from_rgb(147, 197, 253),
                    Color32::from_rgb(15, 23, 42),
                    Color32::from_rgb(30, 58, 138),
                );
                render_status_pill(
                    ui,
                    &format!("~{} direct emails", est_emails),
                    Color32::from_rgb(74, 222, 128),
                    Color32::from_rgb(5, 46, 22),
                    Color32::from_rgb(22, 101, 52),
                );
                render_status_pill(
                    ui,
                    &format!("MX reachability: ~{}%", est_mx),
                    Color32::from_rgb(251, 191, 36),
                    Color32::from_rgb(45, 26, 12),
                    Color32::from_rgb(146, 64, 14),
                );
            });
        });
    }

    fn render_search_sources(&mut self, ui: &mut egui::Ui) {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new(if self.automatic_business_collection {
                    "Collection pipeline"
                } else {
                    "Search sources"
                })
                .strong()
                .size(15.0)
                .color(Color32::WHITE),
            );
            ui.add_space(14.0);
            if self.automatic_business_collection {
                for (step, title, primary_desc, secondary_desc) in [
                    (
                        "01",
                        "Search sources",
                        "OpenStreetMap, then Brave Search (fallback: DuckDuckGo, Yahoo, Mwmbl)",
                        "Multi-pass discovery skips generic directories & aggregators.",
                    ),
                    (
                        "02",
                        "Crawler & extraction",
                        "12 parallel workers with a 30s per-site budget.",
                        "Extracts direct business emails, phones, Telegram, VK and team members.",
                    ),
                    (
                        "03",
                        "Deduplication",
                        "Automatic clustering by domain, email, phone & tax ID (INN).",
                        "Consolidates duplicate entity records across sources.",
                    ),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(step).monospace().size(11.0).color(TEXT_FAINT));
                        ui.label(RichText::new(title).strong().color(Color32::WHITE));
                    });
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(RichText::new(primary_desc).small().color(TEXT_MUTED))
                            .wrap(),
                    );
                    ui.add(
                        egui::Label::new(RichText::new(secondary_desc).small().color(TEXT_FAINT))
                            .wrap(),
                    );
                    ui.add_space(10.0);
                }
            } else {
                ui.label(
                    RichText::new("Direct sources & networks")
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.filter.search_web, "Web");
                    ui.checkbox(&mut self.filter.search_google_maps, "Maps");
                    ui.checkbox(&mut self.filter.search_telegram, "Telegram");
                    ui.checkbox(&mut self.filter.search_linkedin, "LinkedIn");
                });
                ui.add_space(8.0);
                ui.collapsing("Job boards", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.filter.search_remotive, "Remotive");
                        ui.checkbox(&mut self.filter.search_arbeitnow, "Arbeitnow");
                        ui.checkbox(&mut self.filter.search_remoteok, "RemoteOK");
                        ui.checkbox(&mut self.filter.search_jobicy, "Jobicy");
                        ui.checkbox(&mut self.filter.search_hn, "Hacker News");
                    });
                });
                ui.checkbox(&mut self.filter.crawl_websites, "Explore company websites");
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(10.0);
            if self.automatic_business_collection {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("04").monospace().size(11.0).color(TEXT_FAINT));
                    ui.label(
                        RichText::new("DNS & verification")
                            .strong()
                            .color(Color32::WHITE),
                    );
                });
                ui.add_space(2.0);
            }
            ui.checkbox(&mut self.filter.verify_mx, "Check email domains (MX)");
            ui.add(
                egui::Label::new(
                    RichText::new("Direct DNS query verifies whether target domain accepts mail.")
                        .small()
                        .color(TEXT_MUTED),
                )
                .wrap(),
            );
            if self.automatic_business_collection {
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "● Progress is saved automatically to contacts.json & contacts.csv.",
                        )
                        .small()
                        .color(TEXT_FAINT),
                    )
                    .wrap(),
                );
            }
        });
    }

    fn render_search_tab(&mut self, ui: &mut egui::Ui) {
        workspace_heading(
            ui,
            "Find contacts",
            "Discover businesses and save their published contact details.",
        );
        ui.horizontal_wrapped(|ui| {
            if primary_button(
                ui,
                !self.is_searching && !self.is_mailing,
                if self.is_searching {
                    "Collecting..."
                } else {
                    "Start collection"
                },
            )
            .clicked()
            {
                self.start_search();
                self.active_tab = ActiveTab::Logs;
            }
            if ui.button("Open contacts").clicked() {
                self.active_tab = ActiveTab::Contacts;
            }
            if self.is_searching
                && ui
                    .button(RichText::new("Stop collection").color(ACCENT_ROSE))
                    .clicked()
            {
                self.collection_cancel.store(true, Ordering::Relaxed);
                self.status_message =
                    "Stopping collection. Collected contacts remain saved.".into();
            }
        });
        ui.add_space(16.0);
        egui::ScrollArea::vertical()
            .id_salt("search_content")
            .show(ui, |ui| {
                if ui.available_width() >= 840.0 {
                    ui.columns(2, |columns| {
                        self.render_search_criteria(&mut columns[0], 0.0);
                        self.render_search_sources(&mut columns[1]);
                    });
                } else {
                    self.render_search_criteria(ui, 0.0);
                    ui.add_space(16.0);
                    self.render_search_sources(ui);
                }
                ui.add_space(20.0);
                ui.collapsing("Import from 2GIS", |ui| {
                    ui.label(
                        RichText::new(
                            "Collect organizations in the 2GIS dashboard, then import its export.",
                        )
                        .weak(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                !self.is_searching,
                                egui::Button::new("Open 2GIS dashboard"),
                            )
                            .clicked()
                        {
                            self.launch_2gis_web();
                        }
                        if ui
                            .add_enabled(
                                !self.is_searching,
                                egui::Button::new("Import 2GIS export"),
                            )
                            .clicked()
                        {
                            self.run_2gis_import();
                        }
                        self.file_menu(ui);
                    });
                });
                ui.add_space(8.0);
                ui.collapsing("Chrome crawler (browser assistant)", |ui| {
                    ui.label("1. Install the extension from its project page and configure its model. Copy the task below into its Chrome side panel.");
                    ui.horizontal_wrapped(|ui| {
                        ui.hyperlink_to("Extension and installation", chrome_crawler::PROJECT_URL);
                        if ui.button("Copy collection task").clicked() {
                            ui.ctx().copy_text(chrome_crawler::collection_task(
                                &self.filter,
                                self.automatic_business_collection && self.all_industries,
                            ));
                            self.status_message = "Chrome crawler task copied. Paste it into the extension.".into();
                        }
                    });
                    ui.label("2. Copy the agent's JSON response here, then import. Sources and excerpts are retained for review; new contacts start unselected and unverified.");
                    ui.add(egui::TextEdit::multiline(&mut self.chrome_crawler_input)
                        .hint_text("Paste Chrome crawler JSON response")
                        .desired_width(ui.available_width()).desired_rows(5)
                        .char_limit(5 * 1024 * 1024));
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(
                            !self.is_searching && !self.is_mailing && !self.chrome_crawler_input.trim().is_empty(),
                            egui::Button::new("Import browser contacts"),
                        ).clicked() {
                            self.import_chrome_crawler();
                        }
                        if ui.button("Clear response").clicked() {
                            self.chrome_crawler_input.clear();
                        }
                    });
                    ui.label(RichText::new("The extension sends page content to its configured model provider. Upstream license: CC BY-NC 4.0 (noncommercial). It runs separately from automatic collection.").weak());
                });
                ui.add_space(8.0);
                ui.collapsing("Collect from website URLs", |ui| {
                    ui.label(RichText::new("Enter one company website per line.").weak());
                    ui.add(
                        egui::TextEdit::multiline(&mut self.url_crawl_input)
                            .hint_text("https://example.com")
                            .desired_width(ui.available_width())
                            .desired_rows(4)
                            .margin(egui::vec2(10.0, 8.0)),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                !self.is_searching
                                    && !self.is_mailing
                                    && !self.url_crawl_input.trim().is_empty(),
                                egui::Button::new("Collect website contacts"),
                            )
                            .clicked()
                        {
                            self.start_direct_url_crawl();
                            self.active_tab = ActiveTab::Logs;
                        }
                        if ui.button("Clear list").clicked() {
                            self.url_crawl_input.clear();
                        }
                    });
                });
            });
    }

    fn contact_filters_and_actions(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("score_filter")
                .selected_text(format!("Priority: {}", self.table_score_filter))
                .show_ui(ui, |ui| {
                    for label in ["All", "HOT", "WARM", "MX"] {
                        ui.selectable_value(&mut self.table_score_filter, label.into(), label);
                    }
                });
            egui::ComboBox::from_id_salt("status_filter")
                .selected_text(format!("Status: {}", self.table_status_filter))
                .show_ui(ui, |ui| {
                    for label in ["All", "Pending", "Sent", "Replied"] {
                        ui.selectable_value(&mut self.table_status_filter, label.into(), label);
                    }
                });
            let phone_active = self.only_phone_filter;
            if render_filter_chip(
                ui,
                phone_active,
                "Phone",
                "Filter contacts with phone numbers",
            )
            .clicked()
            {
                self.only_phone_filter = !phone_active;
            }
            let tg_active = self.only_tg_filter;
            if render_filter_chip(ui, tg_active, "Telegram", "Filter contacts with Telegram")
                .clicked()
            {
                self.only_tg_filter = !tg_active;
            }
            let pending_active = self.only_pending_filter;
            if render_filter_chip(ui, pending_active, "Unsent", "Filter unsent contacts").clicked()
            {
                self.only_pending_filter = !pending_active;
            }
            let rcpt_active = self.only_rcpt_filter;
            if render_filter_chip(
                ui,
                rcpt_active,
                "RCPT 250",
                "Filter verified mailboxes (RCPT 250 OK)",
            )
            .clicked()
            {
                self.only_rcpt_filter = !rcpt_active;
            }
            ui.menu_button("More filters", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Category:");
                    for cat in [
                        "All",
                        "Showrooms and brands",
                        "Photo studios",
                        "Model agencies",
                        "Restaurants and HoReCa",
                        "Production and advertising",
                    ] {
                        ui.selectable_value(&mut self.table_category_filter, cat.into(), cat);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.only_personal_filter, "Personal / decision maker");
                    ui.checkbox(&mut self.hide_catch_all_filter, "Exclude catch-all");
                });
            });
            ui.menu_button("Sending actions", |ui| self.mailing_actions(ui));
        });
    }

    fn render_contacts_tab(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 700.0 {
            ui.horizontal(|ui| {
                ui.heading("Contacts");
                ui.label(RichText::new(self.contacts.len().to_string()).weak());
            });
            ui.add_space(4.0);
        } else {
            workspace_heading(
                ui,
                "Contacts",
                &format!(
                    "{} saved contacts. Select a company to view details.",
                    self.contacts.len()
                ),
            );
        }
        ui.horizontal_wrapped(|ui| {
            self.file_menu(ui);
            ui.menu_button("Select contacts", |ui| {
                for label in ["Select all", "Select with MX", "Deselect all"] {
                    if ui.button(label).clicked() {
                        for c in &mut self.contacts {
                            c.selected = match label {
                                "Select all" => true,
                                "Select with MX" => c.mx_valid,
                                _ => false,
                            };
                        }
                        ui.close();
                    }
                }
                if ui.button("Copy selected emails").clicked() {
                    let emails: Vec<_> = self
                        .contacts
                        .iter()
                        .filter(|c| c.selected && !c.email.trim().is_empty())
                        .map(|c| c.email.as_str())
                        .collect();
                    ui.ctx().copy_text(emails.join(", "));
                    self.status_message = format!("Copied {} emails.", emails.len());
                    self.show_toast(format!("Copied {} emails", emails.len()), true);
                    ui.close();
                }
            });
            ui.menu_button("Database tools", |ui| {
                if ui
                    .add_enabled(
                        !self.is_searching && !self.is_mailing,
                        egui::Button::new("Enrich contact details"),
                    )
                    .clicked()
                {
                    self.start_waterfall_enrichment();
                    self.active_tab = ActiveTab::Logs;
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.is_searching && !self.is_mailing,
                        egui::Button::new("Merge duplicates"),
                    )
                    .clicked()
                {
                    let (merged, stats) = crate::search::waterfall_merge::deduplicate_and_merge(
                        self.contacts.clone(),
                    );
                    match save_contacts_to_json(&merged, &self.default_json_path) {
                        Ok(()) => {
                            self.contacts = merged;
                            self.status_message = format!(
                                "Merged {} duplicates, enriched {} fields.",
                                stats.duplicates_merged, stats.fields_enriched
                            );
                            self.add_log("SUCCESS", &self.status_message.clone());
                            self.show_toast(
                                format!("Merged {} duplicates", stats.duplicates_merged),
                                true,
                            );
                        }
                        Err(e) => {
                            self.status_message = e.clone();
                            self.add_log("ERROR", &e);
                        }
                    }
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        !self.is_searching && !self.is_mailing,
                        egui::Button::new(
                            RichText::new("Delete all contacts").color(ui.visuals().error_fg_color),
                        ),
                    )
                    .clicked()
                {
                    self.confirm_clear = true;
                    ui.close();
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let density_text = if self.compact_density {
                    "Density: Compact"
                } else {
                    "Density: Relaxed"
                };
                let tip = if self.compact_density {
                    "Switch to relaxed row spacing (8px)"
                } else {
                    "Switch to compact spreadsheet density (3px padding, +40% rows)"
                };
                if ui
                    .button(RichText::new(density_text).small())
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.compact_density = !self.compact_density;
                    let mode = if self.compact_density {
                        "Compact (+40% rows visible)"
                    } else {
                        "Relaxed"
                    };
                    self.show_toast(format!("Table density: {mode}"), true);
                }
            });
        });
        ui.add_space(if ui.available_width() < 700.0 {
            4.0
        } else {
            12.0
        });
        let search_id = egui::Id::new("contacts_search_input");
        if ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Slash)
        }) {
            ui.memory_mut(|m| m.request_focus(search_id));
        }

        ui.horizontal_wrapped(|ui| {
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.table_search_query)
                                .id(search_id)
                                .frame(egui::Frame::NONE)
                                .desired_width(200.0)
                                .hint_text("Search company, email or name…"),
                        );
                        if !self.table_search_query.is_empty() {
                            if ui
                                .small_button(RichText::new("×").size(10.0).color(TEXT_MUTED))
                                .on_hover_text("Clear search")
                                .clicked()
                            {
                                self.table_search_query.clear();
                            }
                        }
                        render_kbd(ui, "/");
                    });
                });
        });
        if let Some(selected_id) = self.selected_contact_id.clone() {
            if let Some(selected) = self.contacts.iter().find(|c| c.id == selected_id) {
                let email = selected.email.clone();
                let url = selected.url.clone();
                let company = selected.company.clone();
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("Selected: {company}"))
                            .small()
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    if !email.is_empty() {
                        if ui
                            .small_button("Copy email")
                            .on_hover_text("Copy email to clipboard")
                            .clicked()
                        {
                            ui.ctx().copy_text(email.clone());
                            self.status_message = format!("Copied email: {email}");
                            self.show_toast(format!("Copied email: {email}"), true);
                        }
                    }
                    if !url.is_empty() {
                        if ui
                            .small_button("Website")
                            .on_hover_text("Open company website")
                            .clicked()
                        {
                            let _ = std::process::Command::new("cmd")
                                .args(["/C", "start", "", &url])
                                .spawn();
                        }
                    }
                    if !email.is_empty() {
                        if ui
                            .small_button("Direct Outreach")
                            .on_hover_text("Preview and send email directly to this contact")
                            .clicked()
                        {
                            self.open_quick_outreach(&selected_id);
                        }
                    }
                    if ui
                        .small_button("Template")
                        .on_hover_text("Switch to Email Template")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Template;
                    }
                });
            }
        }

        let pending_count = self
            .contacts
            .iter()
            .filter(|c| {
                c.status == SendStatus::Pending
                    && !c.email.trim().is_empty()
                    && !self.suppression_list.is_suppressed(&c.email)
            })
            .count();
        let sent_count = self
            .contacts
            .iter()
            .filter(|c| c.status == SendStatus::Sent)
            .count();
        let failed_count = self
            .contacts
            .iter()
            .filter(|c| matches!(c.status, SendStatus::Failed(_)))
            .count();

        if ui.ctx().content_rect().height() >= 480.0 {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("OUTREACH PIPELINE")
                                .small()
                                .color(TEXT_FAINT)
                                .strong(),
                        );
                        ui.label(RichText::new("·").small().color(TEXT_FAINT));
                        render_status_pill(
                            ui,
                            &format!("Pending: {pending_count}"),
                            ACCENT_AMBER,
                            Color32::from_rgb(36, 26, 14),
                            Color32::from_rgb(64, 44, 20),
                        );
                        render_status_pill(
                            ui,
                            &format!("Sent: {sent_count}"),
                            ACCENT_BLUE,
                            Color32::from_rgb(15, 23, 42),
                            Color32::from_rgb(30, 58, 138),
                        );
                        if failed_count > 0 {
                            render_status_pill(
                                ui,
                                &format!("Failed: {failed_count}"),
                                ACCENT_ROSE,
                                Color32::from_rgb(36, 14, 18),
                                Color32::from_rgb(64, 22, 28),
                            );
                        }

                        ui.add_space(8.0);
                        if ui
                            .small_button(format!("Select Unsent ({pending_count})"))
                            .on_hover_text("Select all pending contacts with valid email ready to send")
                            .clicked()
                        {
                            for c in &mut self.contacts {
                                c.selected = c.status == SendStatus::Pending
                                    && !c.email.trim().is_empty()
                                    && !self.suppression_list.is_suppressed(&c.email);
                            }
                            self.show_toast(format!("Selected {pending_count} pending contacts"), true);
                        }

                        let selected_pending = self
                            .contacts
                            .iter()
                            .filter(|c| {
                                c.selected
                                    && c.status == SendStatus::Pending
                                    && !self.suppression_list.is_suppressed(&c.email)
                            })
                            .count();
                        if selected_pending > 0 {
                            let send_label = if self.is_mailing {
                                "Sending campaign...".to_string()
                            } else {
                                format!("Send Campaign ({selected_pending})")
                            };
                            if ui
                                .add_enabled(
                                    !self.is_mailing && !self.is_searching,
                                    egui::Button::new(
                                        RichText::new(send_label)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(37, 99, 235)),
                                )
                                .clicked()
                            {
                                self.start_mailing();
                                self.active_tab = ActiveTab::Logs;
                            }
                        }

                        if failed_count > 0 {
                            if ui
                                .small_button(format!("Reset Failed to Pending ({failed_count})"))
                                .on_hover_text(
                                    "Reset failed contacts back to Pending so they can be re-sent via external SMTP",
                                )
                                .clicked()
                            {
                                let mut count = 0;
                                for c in &mut self.contacts {
                                    if matches!(c.status, SendStatus::Failed(_)) {
                                        c.status = SendStatus::Pending;
                                        count += 1;
                                    }
                                }
                                let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
                                self.show_toast(format!("Reset {count} contacts to Pending"), true);
                            }
                        }
                    });
                });
        }
        if ui.available_width() >= 700.0 {
            let selected_count = self.contacts.iter().filter(|c| c.selected).count();
            if selected_count > 0 {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    render_status_pill(
                        ui,
                        &format!("Selected: {} of {}", selected_count, self.contacts.len()),
                        Color32::from_rgb(96, 165, 250),
                        Color32::from_rgb(15, 23, 42),
                        Color32::from_rgb(30, 58, 138),
                    );
                    if ui.small_button("Deselect all").clicked() {
                        for c in &mut self.contacts {
                            c.selected = false;
                        }
                        self.show_toast("Deselected all contacts", true);
                    }
                    if ui.small_button("Copy selected emails").clicked() {
                        let emails: Vec<_> = self
                            .contacts
                            .iter()
                            .filter(|c| c.selected && !c.email.trim().is_empty())
                            .map(|c| c.email.as_str())
                            .collect();
                        ui.ctx().copy_text(emails.join(", "));
                        self.show_toast(format!("Copied {} emails", emails.len()), true);
                    }
                });
            }

            let total_count = self.contacts.len();
            let dm_count = self
                .contacts
                .iter()
                .filter(|c| !c.is_role_account && !c.email.trim().is_empty())
                .count();
            let mx_count = self
                .contacts
                .iter()
                .filter(|c| c.mx_valid || c.rcpt_verified)
                .count();
            let hot_count = self
                .contacts
                .iter()
                .filter(|c| c.score == LeadScore::Hot)
                .count();

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Quick view:").small().color(TEXT_FAINT));
                let all_active = self.quick_segment_filter.is_none();
                if render_filter_chip(
                    ui,
                    all_active,
                    &format!("All · {total_count}"),
                    "Show all contacts",
                )
                .clicked()
                {
                    self.quick_segment_filter = None;
                }
                let dm_active = self.quick_segment_filter.as_deref() == Some("DM");
                if render_filter_chip(
                    ui,
                    dm_active,
                    &format!("Decision Makers · {dm_count}"),
                    "Show personal decision-maker addresses",
                )
                .clicked()
                {
                    self.quick_segment_filter = if dm_active { None } else { Some("DM".into()) };
                }
                let mx_active = self.quick_segment_filter.as_deref() == Some("MX");
                if render_filter_chip(
                    ui,
                    mx_active,
                    &format!("MX Verified · {mx_count}"),
                    "Show contacts with active mail server",
                )
                .clicked()
                {
                    self.quick_segment_filter = if mx_active { None } else { Some("MX".into()) };
                }
                let hot_active = self.quick_segment_filter.as_deref() == Some("HOT");
                if render_filter_chip(
                    ui,
                    hot_active,
                    &format!("Hot Leads · {hot_count}"),
                    "Show high-scoring leads",
                )
                .clicked()
                {
                    self.quick_segment_filter = if hot_active { None } else { Some("HOT".into()) };
                }
            });

            if total_count > 0 {
                let direct_count = self
                    .contacts
                    .iter()
                    .filter(|c| !c.phone.trim().is_empty() || !c.telegram.trim().is_empty())
                    .count();
                let mx_pct = (mx_count * 100) / total_count.max(1);
                let dm_pct = (dm_count * 100) / total_count.max(1);
                let direct_pct = (direct_count * 100) / total_count.max(1);

                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("PIPELINE QUALITY")
                                    .small()
                                    .color(TEXT_FAINT)
                                    .strong(),
                            );
                            ui.label(RichText::new("·").small().color(TEXT_FAINT));
                            ui.label(
                                RichText::new("●")
                                    .small()
                                    .color(Color32::from_rgb(52, 211, 153)),
                            );
                            ui.label(
                                RichText::new(format!("MX: {mx_pct}%"))
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                            ui.label(RichText::new("·").small().color(TEXT_FAINT));
                            ui.label(
                                RichText::new("●")
                                    .small()
                                    .color(Color32::from_rgb(251, 191, 36)),
                            );
                            ui.label(
                                RichText::new(format!("Decision Makers: {dm_pct}%"))
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                            ui.label(RichText::new("·").small().color(TEXT_FAINT));
                            ui.label(
                                RichText::new("●")
                                    .small()
                                    .color(Color32::from_rgb(96, 165, 250)),
                            );
                            ui.label(
                                RichText::new(format!("Phone/TG: {direct_pct}%"))
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                        });
                        ui.add_space(3.0);
                        let (bar_rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 3.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(bar_rect, 1.5, Color32::from_rgb(24, 24, 27));
                        let mx_w = bar_rect.width() * (mx_pct as f32 / 100.0);
                        if mx_w > 0.0 {
                            let mx_rect =
                                egui::Rect::from_min_size(bar_rect.min, Vec2::new(mx_w, 3.0));
                            painter.rect_filled(mx_rect, 1.5, Color32::from_rgb(16, 185, 129));
                        }
                    });
            }
        }
        if ui.available_width() < 700.0 {
            egui::CollapsingHeader::new("Filters and email actions")
                .show(ui, |ui| self.contact_filters_and_actions(ui));
        } else {
            self.contact_filters_and_actions(ui);
        }
        ui.add_space(if ui.available_width() < 700.0 {
            4.0
        } else {
            12.0
        });
        ui.separator();
        if self.contacts.is_empty() {
            card_frame(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(28.0);
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                    let center = icon_rect.center();
                    ui.painter().circle_stroke(center, 18.0, Stroke::new(1.5, Color32::from_rgb(63, 63, 70)));
                    ui.painter().circle_stroke(center, 10.0, Stroke::new(1.0, Color32::from_rgb(39, 39, 42)));
                    ui.painter().line_segment(
                        [egui::pos2(center.x - 22.0, center.y), egui::pos2(center.x + 22.0, center.y)],
                        Stroke::new(1.0, Color32::from_rgb(63, 63, 70)),
                    );
                    ui.painter().line_segment(
                        [egui::pos2(center.x, center.y - 22.0), egui::pos2(center.x, center.y + 22.0)],
                        Stroke::new(1.0, Color32::from_rgb(63, 63, 70)),
                    );
                    ui.painter().circle_filled(center, 2.5, Color32::WHITE);
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("No contacts in workspace yet")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Start automated discovery in Find contacts, or load a database from File and export.",
                            )
                            .color(TEXT_MUTED),
                        )
                        .wrap(),
                    );
                    ui.add_space(18.0);
                    if primary_button(ui, true, "Find contacts").clicked() {
                        self.active_tab = ActiveTab::Search;
                    }
                    ui.add_space(24.0);
                });
            });
            return;
        }

        let mut clicked_contact_id: Option<Option<String>> = None;
        let mut clicked_quick_outreach_id: Option<String> = None;

        // Table of contacts
        let table_width = ui.available_width().max(680.0);
        let widths = [
            28.0,
            (table_width - 370.0) * 0.44,
            (table_width - 370.0) * 0.56,
            74.0,
            104.0,
            104.0,
        ];
        let mut visible_count = 0;
        let mut triggered_sort: Option<ContactSortField> = None;
        egui::ScrollArea::both()
            .id_salt("contact_rows")
            // The table can be wider than the window; keep the sideways scrollbar visible.
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .max_height((ui.ctx().content_rect().height() * 0.60).max(140.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("contacts_table")
                    .striped(true)
                    .spacing(if self.compact_density {
                        [12.0, 3.0]
                    } else {
                        [12.0, 8.0]
                    })
                    .show(ui, |ui| {
                        for (label, width) in [
                            "",
                            "Company / role",
                            "Contact",
                            "Score",
                            "Email checks",
                            "Status",
                        ]
                        .into_iter()
                        .zip(widths)
                        {
                            let target_field = match label {
                                "Company / role" => Some(ContactSortField::Company),
                                "Contact" => Some(ContactSortField::Email),
                                "Score" => Some(ContactSortField::Score),
                                _ => None,
                            };
                            let active =
                                target_field.is_some() && target_field == Some(self.contact_sort.0);

                            ui.allocate_ui(Vec2::new(width, 24.0), |ui| {
                                ui.set_width(width);
                                ui.horizontal(|ui| {
                                    let label_color = if active {
                                        Color32::from_rgb(96, 165, 250)
                                    } else {
                                        TEXT_MUTED
                                    };
                                    let sense = if target_field.is_some() {
                                        egui::Sense::click()
                                    } else {
                                        egui::Sense::hover()
                                    };
                                    let resp = ui
                                        .add(
                                            egui::Label::new(
                                                RichText::new(label).small().color(label_color),
                                            )
                                            .truncate()
                                            .sense(sense),
                                        )
                                        .on_hover_text(label);
                                    if active {
                                        let caret = if self.contact_sort.1 { "▲" } else { "▼" };
                                        ui.label(
                                            RichText::new(caret)
                                                .small()
                                                .color(Color32::from_rgb(96, 165, 250)),
                                        );
                                    }
                                    if resp.clicked() {
                                        if let Some(f) = target_field {
                                            triggered_sort = Some(f);
                                        }
                                    }
                                });
                            });
                        }
                        ui.end_row();

                        if let Some(field) = triggered_sort {
                            if self.contact_sort.0 == field {
                                if self.contact_sort.1 {
                                    self.contact_sort = (ContactSortField::Default, false);
                                } else {
                                    self.contact_sort.1 = true;
                                }
                            } else {
                                self.contact_sort = (field, false);
                            }
                            self.apply_sort();
                            let sort_name = match self.contact_sort.0 {
                                ContactSortField::Score => {
                                    if self.contact_sort.1 {
                                        "Score (lowest first)"
                                    } else {
                                        "Score (highest first)"
                                    }
                                }
                                ContactSortField::Company => {
                                    if self.contact_sort.1 {
                                        "Company (A-Z)"
                                    } else {
                                        "Company (Z-A)"
                                    }
                                }
                                ContactSortField::Email => {
                                    if self.contact_sort.1 {
                                        "Email (A-Z)"
                                    } else {
                                        "Email (Z-A)"
                                    }
                                }
                                ContactSortField::Default => "Default order",
                            };
                            self.show_toast(format!("Sorted: {sort_name}"), true);
                        }

                        let search_q = self.table_search_query.trim().to_lowercase();
                        let score_flt = self.table_score_filter.clone();
                        let cat_flt = self.table_category_filter.clone();
                        let stat_flt = self.table_status_filter.clone();
                        let current_selected_id = self.selected_contact_id.clone();
                        let mut row_pending_toast: Option<(String, bool)> = None;

                        for contact in &mut self.contacts {
                            // Apply segment filters
                            if self.only_phone_filter && contact.phone.trim().is_empty() {
                                continue;
                            }
                            if self.only_tg_filter && contact.telegram.trim().is_empty() {
                                continue;
                            }
                            if self.only_pending_filter && contact.status != SendStatus::Pending {
                                continue;
                            }
                            if self.only_rcpt_filter && !contact.rcpt_verified {
                                continue;
                            }
                            if self.only_personal_filter && contact.is_role_account {
                                continue;
                            }
                            if self.hide_catch_all_filter && contact.is_catch_all {
                                continue;
                            }
                            if let Some(ref qsf) = self.quick_segment_filter {
                                match qsf.as_str() {
                                    "DM" => {
                                        if contact.is_role_account
                                            || contact.email.trim().is_empty()
                                        {
                                            continue;
                                        }
                                    }
                                    "MX" => {
                                        if !contact.mx_valid && !contact.rcpt_verified {
                                            continue;
                                        }
                                    }
                                    "HOT" => {
                                        if contact.score != LeadScore::Hot {
                                            continue;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // Apply table text search
                            if !search_q.is_empty() {
                                let match_found = contact.email.to_lowercase().contains(&search_q)
                                    || contact.company.to_lowercase().contains(&search_q)
                                    || contact.name.to_lowercase().contains(&search_q)
                                    || contact.role.to_lowercase().contains(&search_q)
                                    || contact.stack.to_lowercase().contains(&search_q)
                                    || contact.source.to_lowercase().contains(&search_q)
                                    || contact.notes.to_lowercase().contains(&search_q);
                                if !match_found {
                                    continue;
                                }
                            }

                            // Apply score filter
                            match score_flt.as_str() {
                                "HOT" => {
                                    if contact.score != LeadScore::Hot {
                                        continue;
                                    }
                                }
                                "WARM" => {
                                    if contact.score != LeadScore::Warm {
                                        continue;
                                    }
                                }
                                "MX" => {
                                    if !contact.mx_valid {
                                        continue;
                                    }
                                }
                                _ => {}
                            }

                            // Apply category filter
                            if cat_flt != "All" {
                                let text = format!(
                                    "{} {} {}",
                                    contact.company, contact.stack, contact.role
                                )
                                .to_lowercase();
                                let matched = match cat_flt.as_str() {
                                    "Showrooms and brands" => {
                                        text.contains("шоурум")
                                            || text.contains("бренд")
                                            || text.contains("одежд")
                                            || text.contains("fashion")
                                            || text.contains("look")
                                            || text.contains("concept")
                                            || text.contains("polyakova")
                                    }
                                    "Photo studios" => {
                                        text.contains("фотостуд")
                                            || text.contains("студия")
                                            || text.contains("аренд")
                                            || text.contains("light")
                                            || text.contains("moon")
                                            || text.contains("etage")
                                    }
                                    "Model agencies" => {
                                        text.contains("модел")
                                            || text.contains("agency")
                                            || text.contains("агентств")
                                            || text.contains("sigma")
                                            || text.contains("jannet")
                                    }
                                    "Restaurants and HoReCa" => {
                                        text.contains("ресторан")
                                            || text.contains("кафе")
                                            || text.contains("бар")
                                            || text.contains("horeca")
                                            || text.contains("гастро")
                                            || text.contains("bellini")
                                            || text.contains("berrywood")
                                            || text.contains("0.75")
                                    }
                                    "Production and advertising" => {
                                        text.contains("продакшн")
                                            || text.contains("реклам")
                                            || text.contains("медиа")
                                            || text.contains("маркетинг")
                                    }
                                    _ => true,
                                };
                                if !matched {
                                    continue;
                                }
                            }

                            // Apply status filter
                            if stat_flt != "All" {
                                let matched = match stat_flt.as_str() {
                                    "Pending" => contact.status == SendStatus::Pending,
                                    "Sent" => contact.status == SendStatus::Sent,
                                    "Replied" => contact.status == SendStatus::Replied,
                                    _ => true,
                                };
                                if !matched {
                                    continue;
                                }
                            }

                            visible_count += 1;
                            table_cell(ui, widths[0], |ui| {
                                let is_selected_row =
                                    current_selected_id.as_deref() == Some(&contact.id);
                                if is_selected_row {
                                    let r = ui.max_rect();
                                    ui.painter().vline(
                                        r.left() - 4.0,
                                        r.y_range(),
                                        Stroke::new(2.5, Color32::from_rgb(16, 185, 129)),
                                    );
                                }
                                let label = format!("Select {}", contact.company);
                                let response =
                                    ui.checkbox(&mut contact.selected, "").on_hover_text(&label);
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Checkbox,
                                        true,
                                        contact.selected,
                                        &label,
                                    )
                                });
                            });
                            table_cell(ui, widths[1], |ui| {
                                let active = current_selected_id.as_deref() == Some(&contact.id);
                                let company = if contact.company.is_empty() {
                                    "Unnamed"
                                } else {
                                    &contact.company
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(company).strong().color(Color32::WHITE),
                                        )
                                        .selected(active)
                                        .frame(false)
                                        .truncate(),
                                    )
                                    .on_hover_text(format!(
                                        "{}\n{}\nOpen contact details",
                                        contact.company, contact.role
                                    ))
                                    .clicked()
                                {
                                    clicked_contact_id = Some(if active {
                                        None
                                    } else {
                                        Some(contact.id.clone())
                                    });
                                }
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&contact.role).small().color(TEXT_MUTED),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(&contact.role);
                            });
                            table_cell(ui, widths[2], |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&contact.email)
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(&contact.email);
                                let detail = if !contact.phone.is_empty() {
                                    contact.phone.as_str()
                                } else if !contact.telegram.is_empty() {
                                    contact.telegram.as_str()
                                } else if contact.is_role_account {
                                    "Company role address"
                                } else {
                                    "Personal / decision maker"
                                };
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(detail).small().color(TEXT_FAINT),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(detail);
                            });
                            table_cell(ui, widths[3], |ui| {
                                let (label, fg, bg, border) = match contact.score {
                                    LeadScore::Hot => (
                                        "Hot",
                                        ACCENT_ROSE,
                                        Color32::from_rgb(42, 18, 21),
                                        Color32::from_rgb(76, 29, 36),
                                    ),
                                    LeadScore::Warm => (
                                        "Warm",
                                        ACCENT_AMBER,
                                        Color32::from_rgb(36, 26, 14),
                                        Color32::from_rgb(69, 44, 16),
                                    ),
                                    LeadScore::Cold => (
                                        "Cold",
                                        TEXT_MUTED,
                                        Color32::from_rgb(24, 24, 27),
                                        Color32::from_rgb(39, 39, 42),
                                    ),
                                };
                                let text = if contact.lead_score > 0 {
                                    format!("{label} · {}", contact.lead_score)
                                } else {
                                    label.to_string()
                                };
                                render_status_pill(ui, &text, fg, bg, border);
                            });
                            table_cell(ui, widths[4], |ui| {
                                if contact.rcpt_verified {
                                    ui.label(RichText::new("RCPT 250").color(ACCENT_TEAL));
                                } else if contact.mx_valid {
                                    ui.label(RichText::new("MX valid").color(TEXT_COLOR));
                                } else {
                                    ui.label(RichText::new("No MX").small().color(TEXT_FAINT));
                                }
                                let detail = if contact.is_catch_all {
                                    "Catch-All"
                                } else {
                                    contact.mail_provider.as_str()
                                };
                                if !detail.is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(detail).small().color(TEXT_FAINT),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(detail);
                                }
                            });
                            table_cell(ui, widths[5], |ui| {
                                let (status_text, status_color) = match contact.status {
                                    SendStatus::Pending => ("Pending", TEXT_MUTED),
                                    SendStatus::Sent => ("Sent", ACCENT_BLUE),
                                    SendStatus::Replied => ("Replied", ACCENT_TEAL),
                                    SendStatus::Skipped => ("Skipped", TEXT_FAINT),
                                    SendStatus::Failed(_) => ("Failed", ACCENT_ROSE),
                                };
                                ui.label(RichText::new(status_text).color(status_color));
                                ui.scope(|ui| {
                                    ui.spacing_mut().interact_size.y = 20.0;
                                    ui.spacing_mut().button_padding = Vec2::new(7.0, 1.5);
                                    ui.horizontal(|ui| {
                                        ui.menu_button("Actions ▾", |ui| {
                                            if !contact.email.trim().is_empty() {
                                                if ui.button("Direct Outreach").clicked() {
                                                    clicked_quick_outreach_id =
                                                        Some(contact.id.clone());
                                                    ui.close();
                                                }
                                            }
                                            if ui.button("Open details").clicked() {
                                                clicked_contact_id = Some(Some(contact.id.clone()));
                                                ui.close();
                                            }
                                            if ui.button("Copy email").clicked() {
                                                ui.ctx().copy_text(contact.email.clone());
                                                row_pending_toast = Some((
                                                    format!("Copied email: {}", contact.email),
                                                    true,
                                                ));
                                                ui.close();
                                            }
                                            if contact.status == SendStatus::Sent
                                                && ui.button("Mark as replied").clicked()
                                            {
                                                contact.status = SendStatus::Replied;
                                                ui.close();
                                            }
                                            if contact.status == SendStatus::Replied
                                                && ui.button("Reset status").clicked()
                                            {
                                                contact.status = SendStatus::Pending;
                                                ui.close();
                                            }
                                            ui.separator();
                                            if ui
                                                .add_enabled(
                                                    !self.is_searching && !self.is_mailing,
                                                    egui::Button::new(
                                                        RichText::new("Delete contact")
                                                            .color(ui.visuals().error_fg_color),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                self.pending_delete = Some(contact.id.clone());
                                                ui.close();
                                            }
                                        });
                                    });
                                });
                            });

                            ui.end_row();
                        }

                        if let Some((msg, is_ok)) = row_pending_toast {
                            self.show_toast(msg, is_ok);
                        }
                    });
            });

        if visible_count == 0 {
            ui.label("No matches. Change the query or filters.");
        }

        if let Some(new_sel) = clicked_contact_id {
            self.selected_contact_id = new_sel;
        }

        if let Some(qid) = clicked_quick_outreach_id {
            self.open_quick_outreach(&qid);
        }

        let selected_count = self.contacts.iter().filter(|c| c.selected).count();
        if selected_count > 0 && ui.available_width() >= 640.0 {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(18, 18, 22))
                .stroke(Stroke::new(1.0, Color32::from_rgb(39, 39, 42)))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        render_status_pill(
                            ui,
                            &format!("● {selected_count} leads selected"),
                            Color32::from_rgb(96, 165, 250),
                            Color32::from_rgb(15, 23, 42),
                            Color32::from_rgb(30, 58, 138),
                        );
                        ui.label(RichText::new("·").color(TEXT_FAINT));
                        if ui
                            .button("Copy emails")
                            .on_hover_text("Copy selected emails to clipboard")
                            .clicked()
                        {
                            let emails: Vec<_> = self
                                .contacts
                                .iter()
                                .filter(|c| c.selected && !c.email.trim().is_empty())
                                .map(|c| c.email.as_str())
                                .collect();
                            ui.ctx().copy_text(emails.join(", "));
                            self.show_toast(format!("Copied {} emails", emails.len()), true);
                        }
                        if ui
                            .button("Mark Hot")
                            .on_hover_text("Promote selected leads to Hot (Score 90)")
                            .clicked()
                        {
                            for c in &mut self.contacts {
                                if c.selected {
                                    c.score = LeadScore::Hot;
                                    c.lead_score = c.lead_score.max(85);
                                }
                            }
                            let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
                            self.show_toast(format!("Marked {selected_count} leads as Hot"), true);
                        }
                        if ui
                            .button("Mark Pending")
                            .on_hover_text("Reset selected leads to Pending")
                            .clicked()
                        {
                            for c in &mut self.contacts {
                                if c.selected {
                                    c.status = SendStatus::Pending;
                                }
                            }
                            let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
                            self.show_toast(
                                format!("Marked {selected_count} leads as Pending"),
                                true,
                            );
                        }
                        if ui
                            .button(RichText::new("Deselect all").color(TEXT_MUTED))
                            .clicked()
                        {
                            for c in &mut self.contacts {
                                c.selected = false;
                            }
                            self.show_toast("Deselected all leads", true);
                        }
                    });
                });
        }

        // CRM Contact Inspector Drawer
        if let Some(ref sel_id) = self.selected_contact_id.clone() {
            let mut save_needed = false;
            let mut should_close = false;
            let mut inspector_open = true;
            let mut blacklist_target: Option<(String, String)> = None;
            let mut pending_toast: Option<(String, bool)> = None;
            let mut direct_outreach_target: Option<String> = None;

            if let Some(contact) = self.contacts.iter_mut().find(|c| c.id == *sel_id) {
                ui.add_space(8.0);
                egui::Window::new("Contact details")
                    .open(&mut inspector_open)
                    .id(egui::Id::new("contact_inspector"))
                    .default_width(720.0)
                    .max_width((ui.ctx().content_rect().width() - 60.0).max(300.0))
                    .max_height((ui.ctx().content_rect().height() - 80.0).max(200.0))
                    .collapsible(false)
                    .vscroll(true)
                    .show(ui.ctx(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("Lead details: {}", contact.company))
                                .strong()
                                .size(16.0)
                                .color(Color32::WHITE),
                        );
                        if !contact.email.is_empty() {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Direct Outreach")
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(37, 99, 235)),
                                )
                                .on_hover_text("Preview and send an email directly to this contact")
                                .clicked()
                            {
                                direct_outreach_target = Some(contact.id.clone());
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close details").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                should_close = true;
                            }
                            render_kbd(ui, "Esc");
                        });
                    });

                    ui.add_space(6.0);

                    egui::Grid::new("crm_inspector_grid")
                        .max_col_width((ui.available_width() - 190.0).max(140.0))
                        .num_columns(2)
                        .spacing([14.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Company / contact:").strong());
                            ui.label(format!("{} | {} ({})", contact.company, contact.name, contact.role));
                            ui.end_row();

                            ui.label(RichText::new("Email and MX:").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&contact.email).strong().color(ui.visuals().text_color()));
                                if ui.small_button("Copy").clicked() {
                                    ui.ctx().copy_text(contact.email.clone());
                                    pending_toast = Some((format!("Copied email: {}", contact.email), true));
                                }
                                let mx_label = if contact.mx_valid {
                                    RichText::new("MX active").color(success_color(ui))
                                } else {
                                    RichText::new("MX not confirmed").color(ui.visuals().weak_text_color())
                                };
                                ui.label(mx_label);

                                if contact.rcpt_verified {
                                    render_status_pill(ui, "RCPT 250 OK", ACCENT_TEAL, Color32::from_rgb(10, 32, 24), Color32::from_rgb(18, 64, 46));
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Lead score:").strong());
                            ui.horizontal_wrapped(|ui| {
                                let (score_fg, score_bg, score_border) = if contact.lead_score >= 70 {
                                    (success_color(ui), Color32::from_rgb(12, 30, 20), Color32::from_rgb(20, 56, 36))
                                } else if contact.lead_score >= 40 {
                                    (ui.visuals().warn_fg_color, Color32::from_rgb(36, 26, 14), Color32::from_rgb(64, 44, 20))
                                } else {
                                    (ui.visuals().error_fg_color, Color32::from_rgb(36, 14, 18), Color32::from_rgb(64, 22, 28))
                                };
                                render_status_pill(ui, &format!("Score: {}/100", contact.lead_score), score_fg, score_bg, score_border);
                                if contact.is_role_account {
                                    render_status_pill(ui, "Role Address", ui.visuals().error_fg_color, Color32::from_rgb(36, 14, 18), Color32::from_rgb(64, 22, 28));
                                } else {
                                    render_status_pill(ui, "Decision Maker", success_color(ui), Color32::from_rgb(12, 30, 20), Color32::from_rgb(20, 56, 36));
                                }
                                if contact.is_catch_all {
                                    render_status_pill(ui, "Catch-All", ui.visuals().warn_fg_color, Color32::from_rgb(36, 26, 14), Color32::from_rgb(64, 44, 20));
                                }
                                if !contact.mail_provider.is_empty() {
                                    render_status_pill(ui, &contact.mail_provider, ui.visuals().hyperlink_color, Color32::from_rgb(14, 26, 38), Color32::from_rgb(24, 46, 70));
                                }
                            });
                            ui.end_row();

                            if let Some(fix) = contact.suggested_email_fix.clone() {
                                ui.label(RichText::new("Typo Guard:").strong().color(ui.visuals().error_fg_color));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(format!("Possible domain typo. Suggested correction: {}", fix)).color(ui.visuals().warn_fg_color));
                                    if ui.button("Apply correction").clicked() {
                                        contact.email = fix;
                                        contact.suggested_email_fix = None;
                                        save_needed = true;
                                    }
                                });
                                ui.end_row();
                            }

                            if !contact.tech_stack.is_empty() {
                                ui.label(RichText::new("Technology stack:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    for tag in &contact.tech_stack {
                                        render_status_pill(ui, tag, success_color(ui), Color32::from_rgb(14, 28, 20), Color32::from_rgb(24, 52, 36));
                                    }
                                });
                                ui.end_row();
                            }

                            if !contact.b2b_tier.is_empty() || !contact.legal_name.is_empty() || !contact.company_description.is_empty() || !contact.address.is_empty() {
                                ui.label(RichText::new("Company information (B2B):").strong());
                                ui.vertical(|ui| {
                                    if !contact.b2b_tier.is_empty() {
                                        let tier_col = if contact.b2b_tier == "Tier 1" { success_color(ui) } else { ui.visuals().hyperlink_color };
                                        render_status_pill(ui, &contact.b2b_tier, tier_col, Color32::from_rgb(14, 26, 38), Color32::from_rgb(24, 46, 70));
                                    }
                                    if !contact.legal_name.is_empty() {
                                        ui.label(RichText::new(format!("Legal name: {}", contact.legal_name)).color(ui.visuals().text_color()));
                                    }
                                    if !contact.company_description.is_empty() {
                                        ui.label(RichText::new(format!("About: {}", contact.company_description)).color(ui.visuals().weak_text_color()).small());
                                    }
                                    if !contact.address.is_empty() {
                                        ui.label(RichText::new(format!("Address: {}", contact.address)).color(ui.visuals().weak_text_color()).small());
                                    }
                                });
                                ui.end_row();
                            }

                            if contact.domain_age_days.is_some() || !contact.domain_pattern.is_empty() || !contact.domain_registrar.is_empty() {
                                ui.label(RichText::new("Domain intelligence:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    if let Some(days) = contact.domain_age_days {
                                        let yrs = days as f64 / 365.25;
                                        let age_text = format!("Age: {:.1} years ({} days)", yrs, days);
                                        let age_col = if days < 60 { ui.visuals().error_fg_color } else if days > 1095 { success_color(ui) } else { ui.visuals().weak_text_color() };
                                        ui.label(RichText::new(age_text).color(age_col));
                                    }
                                    if !contact.domain_registrar.is_empty() {
                                        ui.label(RichText::new(format!("Registrar: {}", contact.domain_registrar)).color(ui.visuals().weak_text_color()));
                                    }
                                    if !contact.domain_pattern.is_empty() {
                                        ui.label(RichText::new(format!("Pattern: {}", contact.domain_pattern)).color(ui.visuals().hyperlink_color));
                                    }
                                });
                                ui.end_row();
                            }

                            if contact.gravatar_found || !contact.avatar_url.is_empty() {
                                ui.label(RichText::new("Gravatar profile:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    render_status_pill(ui, "Avatar confirmed", success_color(ui), Color32::from_rgb(14, 28, 20), Color32::from_rgb(24, 52, 36));
                                    ui.label(RichText::new(&contact.avatar_url).color(ui.visuals().hyperlink_color).small());
                                });
                                ui.end_row();
                            }

                            ui.label(RichText::new("Phone and messengers:").strong());
                            ui.horizontal_wrapped(|ui| {
                                if !contact.phone.is_empty() {
                                    ui.label(RichText::new(&contact.phone).color(ui.visuals().text_color()).strong());
                                    if ui.small_button("Copy phone").clicked() {
                                        ui.ctx().copy_text(contact.phone.clone());
                                        pending_toast = Some((format!("Copied phone: {}", contact.phone), true));
                                    }
                                    if !contact.phone_carrier.is_empty() {
                                        render_status_pill(ui, &format!("{}: {}", contact.phone_carrier, contact.phone_region), ui.visuals().hyperlink_color, Color32::from_rgb(14, 26, 38), Color32::from_rgb(24, 46, 70));
                                    }
                                    if !contact.phone_type.is_empty() {
                                        ui.label(RichText::new(format!("({})", contact.phone_type)).color(ui.visuals().weak_text_color()).small());
                                    }
                                } else {
                                    ui.label(RichText::new("No phone number").color(ui.visuals().weak_text_color()));
                                }

                                if !contact.whatsapp_url.is_empty() {
                                    if ui.button(RichText::new("WhatsApp").color(ui.visuals().text_color())).clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &contact.whatsapp_url])
                                            .spawn();
                                    }
                                } else if !contact.whatsapp.is_empty() {
                                    let wa_clean = contact.whatsapp.trim_start_matches('+');
                                    let wa_url = format!("https://wa.me/{}", wa_clean);
                                    if ui.button(RichText::new("WhatsApp").color(ui.visuals().text_color())).clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &wa_url])
                                            .spawn();
                                    }
                                }

                                if !contact.telegram_url.is_empty() {
                                    if ui.button(RichText::new("Telegram").color(ui.visuals().text_color())).clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &contact.telegram_url])
                                            .spawn();
                                    }
                                } else if !contact.telegram.is_empty() {
                                    let tg_clean = contact.telegram.trim_start_matches('@').trim_start_matches("https://t.me/").trim_start_matches("t.me/");
                                    if ui.button(RichText::new(format!("Telegram (@{})", tg_clean)).color(ui.visuals().text_color())).clicked() {
                                        let url = format!("https://t.me/{}", tg_clean);
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &url])
                                            .spawn();
                                    }
                                    if ui.small_button("Copy TG").clicked() {
                                        ui.ctx().copy_text(contact.telegram.clone());
                                        pending_toast = Some((format!("Copied Telegram: {}", contact.telegram), true));
                                    }
                                }

                                if !contact.vk_url.is_empty() {
                                    if ui.button(RichText::new("VK").color(ui.visuals().text_color())).clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &contact.vk_url])
                                            .spawn();
                                    }
                                }
                            });
                            ui.end_row();

                            if !contact.github_url.is_empty() {
                                ui.label(RichText::new("GitHub OSINT:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    render_status_pill(ui, "Developer Profile", success_color(ui), Color32::from_rgb(14, 28, 20), Color32::from_rgb(24, 52, 36));
                                    if !contact.github_bio.is_empty() {
                                        ui.label(RichText::new(format!("\"{}\"", contact.github_bio)).color(ui.visuals().weak_text_color()).small());
                                    }
                                    if ui.button("Open GitHub").clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &contact.github_url])
                                            .spawn();
                                    }
                                });
                                ui.end_row();
                            }

                            if !contact.inn.is_empty() || !contact.ogrn.is_empty() || !contact.vat_id.is_empty() {
                                ui.label(RichText::new("Registration and legal details:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    if !contact.inn.is_empty() {
                                        ui.label(RichText::new(format!("INN: {}", contact.inn)).color(ui.visuals().text_color()).strong());
                                        if ui.small_button("Rusprofile").clicked() {
                                            let url = format!("https://www.rusprofile.ru/search?query={}", contact.inn);
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "start", "", &url])
                                                .spawn();
                                        }
                                    }
                                    if !contact.ogrn.is_empty() {
                                        ui.label(RichText::new(format!("OGRN: {}", contact.ogrn)).color(ui.visuals().weak_text_color()));
                                    }
                                    if !contact.vat_id.is_empty() {
                                        ui.label(RichText::new(format!("VAT: {}", contact.vat_id)).color(ui.visuals().hyperlink_color));
                                    }
                                });
                                ui.end_row();
                            }

                            if contact.hiring_active {
                                ui.label(RichText::new("Hiring signals:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    render_status_pill(ui, "Actively hiring", success_color(ui), Color32::from_rgb(14, 28, 20), Color32::from_rgb(24, 52, 36));
                                    for r in &contact.hiring_roles {
                                        render_status_pill(ui, r, ui.visuals().warn_fg_color, Color32::from_rgb(36, 26, 14), Color32::from_rgb(64, 44, 20));
                                    }
                                });
                                ui.end_row();
                            }

                            if !contact.icebreaker.is_empty() {
                                ui.label(RichText::new("Icebreaker:").strong());
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(format!("\"{}\"", contact.icebreaker)).color(ui.visuals().warn_fg_color).italics());
                                    if ui.small_button("Copy").clicked() {
                                        ui.ctx().copy_text(contact.icebreaker.clone());
                                        pending_toast = Some(("Copied icebreaker to clipboard".into(), true));
                                    }
                                });
                                ui.end_row();
                            }

                            if !contact.compliance_status.is_empty() || contact.compliance_score > 0 {
                                ui.label(RichText::new("Privacy compliance (152-FZ / GDPR):").strong());
                                ui.horizontal_wrapped(|ui| {
                                    let comp_col = if contact.compliance_score >= 70 { success_color(ui) } else { ui.visuals().warn_fg_color };
                                    render_status_pill(ui, &format!("{}/100 {}", contact.compliance_score, contact.compliance_status), comp_col, Color32::from_rgb(14, 26, 38), Color32::from_rgb(24, 46, 70));
                                    if !contact.privacy_policy_url.is_empty() {
                                        if ui.small_button("Policy").clicked() {
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "start", "", &contact.privacy_policy_url])
                                                .spawn();
                                        }
                                    }
                                });
                                ui.end_row();
                            }

                            ui.label(RichText::new("Company website:").strong());
                            ui.horizontal_wrapped(|ui| {
                                if !contact.url.is_empty() {
                                    ui.label(&contact.url);
                                    if ui.small_button("Copy URL").clicked() {
                                        ui.ctx().copy_text(contact.url.clone());
                                        pending_toast = Some((format!("Copied URL: {}", contact.url), true));
                                    }
                                    if ui.button("Open website").clicked() {
                                        let _ = std::process::Command::new("cmd")
                                            .args(["/C", "start", "", &contact.url])
                                            .spawn();
                                    }
                                } else {
                                    ui.label(RichText::new("No website").color(ui.visuals().weak_text_color()));
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Industry tags:").strong());
                            ui.horizontal_wrapped(|ui| {
                                if contact.tags.is_empty() {
                                    ui.label(RichText::new("No tags detected").color(ui.visuals().weak_text_color()));
                                } else {
                                    for tag in &contact.tags {
                                        render_status_pill(ui, tag, success_color(ui), Color32::from_rgb(14, 28, 20), Color32::from_rgb(24, 52, 36));
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Lead status:").strong());
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt("crm_status_select")
                                    .selected_text(format!("{}", contact.status))
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut contact.status, SendStatus::Pending, "Pending").clicked() {
                                            save_needed = true;
                                        }
                                        if ui.selectable_value(&mut contact.status, SendStatus::Sent, "Sent").clicked() {
                                            save_needed = true;
                                        }
                                        if ui.selectable_value(&mut contact.status, SendStatus::Replied, "Replied").clicked() {
                                            save_needed = true;
                                        }
                                        if ui.selectable_value(&mut contact.status, SendStatus::Skipped, "Skipped").clicked() {
                                            save_needed = true;
                                        }
                                    });
                            });
                            ui.end_row();

                            ui.label(RichText::new("Quick triage:").strong());
                            ui.horizontal_wrapped(|ui| {
                                if render_filter_chip(ui, contact.status == SendStatus::Replied, "✔ Interested", "Mark lead as positive response").clicked() {
                                    contact.status = SendStatus::Replied;
                                    contact.score = LeadScore::Hot;
                                    pending_toast = Some((format!("Marked {} as Interested (Hot lead)", contact.name), true));
                                    save_needed = true;
                                }
                                if render_filter_chip(ui, contact.status == SendStatus::Pending, "⏳ Follow-up", "Schedule next outreach follow-up").clicked() {
                                    contact.status = SendStatus::Pending;
                                    pending_toast = Some((format!("Marked {} for Follow-up", contact.name), true));
                                    save_needed = true;
                                }
                                if render_filter_chip(ui, contact.status == SendStatus::Skipped, "✖ Not interested", "Archive and stop further outreach").clicked() {
                                    contact.status = SendStatus::Skipped;
                                    contact.score = LeadScore::Cold;
                                    pending_toast = Some((format!("Marked {} as Not interested", contact.name), false));
                                    save_needed = true;
                                }
                                if render_filter_chip(ui, false, "⚠ Bounced", "Mark contact as bounced / undeliverable").clicked() {
                                    contact.status = SendStatus::Skipped;
                                    contact.mx_valid = false;
                                    pending_toast = Some((format!("Marked {} as Bounced (MX invalid)", contact.name), false));
                                    save_needed = true;
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Suppression:").strong());
                            ui.horizontal_wrapped(|ui| {
                                if ui.add(
                                    egui::Button::new(RichText::new("Add to suppression list").color(ui.visuals().error_fg_color)),
                                ).clicked() {
                                    blacklist_target = Some((contact.email.clone(), contact.company.clone()));
                                    contact.status = SendStatus::Skipped;
                                    save_needed = true;
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Conversation notes:").strong());
                            let note_edit = ui.add(
                                egui::TextEdit::multiline(&mut contact.notes)
                                    .hint_text("Add a note: called, requested a proposal, sent a portfolio...")
                                    .desired_width(ui.available_width() - 20.0)
                                    .desired_rows(3),
                            );
                            if note_edit.changed() {
                                save_needed = true;
                            }
                            ui.end_row();
                        });
                });
            }

            if let Some((msg, ok)) = pending_toast {
                self.show_toast(msg, ok);
            }

            if let Some((email, company)) = blacklist_target {
                if !email.is_empty() {
                    self.suppression_list.add_email(&email);
                }
                if let Some(domain) = email.split('@').nth(1) {
                    if !domain.is_empty() {
                        self.suppression_list.add_domain(domain);
                    }
                }
                let _ = save_suppression_list(&self.suppression_list, &self.suppression_path);
                self.add_log(
                    "WARN",
                    &format!(
                        "Contact {} ({}) added to the suppression list.",
                        email, company
                    ),
                );
                self.status_message = format!("Contact {} suppressed.", email);
            }

            if should_close
                || !inspector_open
                || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.selected_contact_id = None;
            }
            if save_needed {
                let _ = save_contacts_to_json(&self.contacts, &self.default_json_path);
            }
            if let Some(target_id) = direct_outreach_target {
                self.open_quick_outreach(&target_id);
            }
        }
    }

    fn save_template(&mut self) {
        let result = storage::save_email_template(&self.smtp, &self.template, &self.settings_path);
        let (success, message) = match result {
            Ok(()) => (
                true,
                "Template saved locally. Use Load saved template to restore it.".to_string(),
            ),
            Err(error) => (false, format!("Could not save template: {error}")),
        };
        self.status_message = message.clone();
        self.add_log(if success { "SUCCESS" } else { "ERROR" }, &message);
        self.template_notice = Some((success, message));
        self.show_toast(
            if success {
                "Template saved"
            } else {
                "Failed to save template"
            },
            success,
        );
    }

    fn load_saved_template(&mut self) {
        let (success, message) = match storage::load_email_template(&self.settings_path) {
            Ok(template) => {
                self.template = template;
                self.editor_preview_key.clear();
                self.spam_check_result = None;
                self.spintax_samples.clear();
                (true, "Saved template loaded.".to_string())
            }
            Err(error) => (false, error),
        };
        self.status_message = message.clone();
        self.add_log(if success { "SUCCESS" } else { "ERROR" }, &message);
        self.template_notice = Some((success, message));
        self.show_toast(
            if success {
                "Saved template loaded"
            } else {
                "Could not load template"
            },
            success,
        );
    }

    fn render_template_editor(&mut self, ui: &mut egui::Ui) {
        card_frame(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Subject").strong().color(Color32::WHITE));
                ui.add_space(6.0);
                for tag in ["{role}", "{city}", "{name}", "{company}"] {
                    if ui
                        .small_button(RichText::new(tag).size(10.5).color(TEXT_MUTED))
                        .clicked()
                    {
                        self.template.subject.push(' ');
                        self.template.subject.push_str(tag);
                    }
                }
            });
            ui.add_space(4.0);
            let subject_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut self.template.subject)
                        .desired_width(ui.available_width())
                        .margin(egui::vec2(10.0, 8.0)),
                )
                .changed();
            let subj_len = self.template.subject.chars().count();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{subj_len} chars"))
                        .small()
                        .color(TEXT_FAINT),
                );
                if subj_len == 0 {
                    render_status_pill(
                        ui,
                        "Empty subject",
                        TEXT_MUTED,
                        Color32::from_rgb(24, 24, 27),
                        BORDER_COLOR,
                    );
                } else if subj_len < 16 {
                    render_status_pill(
                        ui,
                        "Brief · High urgency",
                        Color32::from_rgb(147, 197, 253),
                        Color32::from_rgb(30, 58, 138),
                        Color32::from_rgb(30, 64, 175),
                    );
                } else if subj_len <= 50 {
                    render_status_pill(
                        ui,
                        "Optimal (20–50) · High open rate",
                        Color32::from_rgb(74, 222, 128),
                        Color32::from_rgb(5, 46, 22),
                        Color32::from_rgb(22, 101, 52),
                    );
                } else {
                    render_status_pill(
                        ui,
                        "Long · Mobile risk",
                        Color32::from_rgb(251, 191, 36),
                        Color32::from_rgb(69, 26, 3),
                        Color32::from_rgb(154, 52, 18),
                    );
                }
            });
            ui.add_space(14.0);
            ui.label(RichText::new("Message").strong().color(Color32::WHITE));
            ui.add_space(4.0);
            let body_changed = ui
                .add(
                    egui::TextEdit::multiline(&mut self.template.body)
                        .desired_width(ui.available_width())
                        .desired_rows(10)
                        .margin(egui::vec2(12.0, 10.0)),
                )
                .changed();
            if subject_changed || body_changed {
                self.template_notice = None;
            }
            let word_count = self.template.body.split_whitespace().count();
            let read_seconds = ((word_count as f32 / 225.0) * 60.0).ceil() as u32;
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{word_count} words · ~{read_seconds}s read"))
                        .small()
                        .color(TEXT_FAINT),
                );
                if word_count == 0 {
                    render_status_pill(
                        ui,
                        "Empty message",
                        TEXT_MUTED,
                        Color32::from_rgb(24, 24, 27),
                        BORDER_COLOR,
                    );
                } else if word_count < 50 {
                    render_status_pill(
                        ui,
                        "Under 50 words · Very brief",
                        Color32::from_rgb(147, 197, 253),
                        Color32::from_rgb(30, 58, 138),
                        Color32::from_rgb(30, 64, 175),
                    );
                } else if word_count <= 125 {
                    render_status_pill(
                        ui,
                        "50–125 words · Optimal reply sweet spot",
                        Color32::from_rgb(74, 222, 128),
                        Color32::from_rgb(5, 46, 22),
                        Color32::from_rgb(22, 101, 52),
                    );
                } else if word_count <= 180 {
                    render_status_pill(
                        ui,
                        "126–180 words · Higher cognitive friction",
                        Color32::from_rgb(251, 191, 36),
                        Color32::from_rgb(69, 26, 3),
                        Color32::from_rgb(154, 52, 18),
                    );
                } else {
                    render_status_pill(
                        ui,
                        "180+ words · Response penalty risk",
                        Color32::from_rgb(248, 113, 113),
                        Color32::from_rgb(69, 10, 10),
                        Color32::from_rgb(153, 27, 27),
                    );
                }
            });
            if word_count > 0 {
                let body_lower = self.template.body.to_lowercase();
                let words: Vec<&str> = body_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| !s.is_empty())
                    .collect();
                let you_count = words
                    .iter()
                    .filter(|&&w| {
                        matches!(
                            w,
                            "you"
                                | "your"
                                | "yours"
                                | "вы"
                                | "вам"
                                | "вас"
                                | "ваш"
                                | "ваша"
                                | "ваше"
                                | "ваши"
                                | "вами"
                        )
                    })
                    .count();
                let i_count = words
                    .iter()
                    .filter(|&&w| {
                        matches!(
                            w,
                            "i" | "my"
                                | "me"
                                | "mine"
                                | "we"
                                | "our"
                                | "ours"
                                | "us"
                                | "я"
                                | "мой"
                                | "моя"
                                | "мое"
                                | "моё"
                                | "мои"
                                | "мне"
                                | "меня"
                                | "мной"
                                | "мы"
                                | "наш"
                                | "наша"
                                | "наше"
                                | "наши"
                                | "нам"
                                | "нас"
                        )
                    })
                    .count();

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("Psychology (You {you_count} : Me {i_count}):"))
                            .small()
                            .color(TEXT_FAINT),
                    );
                    if you_count == 0 && i_count == 0 {
                        render_status_pill(
                            ui,
                            "Neutral focus · No personal pronouns",
                            TEXT_MUTED,
                            Color32::from_rgb(24, 24, 27),
                            BORDER_COLOR,
                        );
                    } else if i_count == 0 {
                        render_status_pill(
                            ui,
                            "Prospect-centric · 100% target focus",
                            Color32::from_rgb(74, 222, 128),
                            Color32::from_rgb(5, 46, 22),
                            Color32::from_rgb(22, 101, 52),
                        );
                    } else {
                        let ratio = you_count as f32 / i_count as f32;
                        if ratio >= 2.0 {
                            render_status_pill(
                                ui,
                                &format!("Prospect-centric · You/I: {:.1}x", ratio),
                                Color32::from_rgb(74, 222, 128),
                                Color32::from_rgb(5, 46, 22),
                                Color32::from_rgb(22, 101, 52),
                            );
                        } else if ratio >= 1.0 {
                            render_status_pill(
                                ui,
                                &format!("Balanced · You/I: {:.1}x", ratio),
                                Color32::from_rgb(56, 189, 248),
                                Color32::from_rgb(12, 44, 73),
                                Color32::from_rgb(14, 116, 144),
                            );
                        } else {
                            render_status_pill(
                                ui,
                                &format!(
                                    "Self-focused · You/I: {:.1}x (focus more on prospect)",
                                    ratio
                                ),
                                Color32::from_rgb(251, 191, 36),
                                Color32::from_rgb(69, 26, 3),
                                Color32::from_rgb(154, 52, 18),
                            );
                        }
                    }
                });
            }
            ui.add_space(10.0);
            let analysis = analyze_template(&self.template.subject, &self.template.body);
            ui.horizontal_wrapped(|ui| {
                let (status, color) = if analysis.valid_syntax {
                    ("Syntax valid", success_color(ui))
                } else {
                    ("Template syntax error", ui.visuals().error_fg_color)
                };
                ui.colored_label(color, status);
                ui.label(
                    RichText::new(format!("About {} variations", analysis.total_combinations))
                        .weak(),
                );
            });
            for error in &analysis.errors {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            for warning in &analysis.warnings {
                ui.colored_label(ui.visuals().warn_fg_color, warning);
            }
            ui.add_space(8.0);
            ui.collapsing("Personalization and variation help", |ui| {
                ui.add(egui::Label::new("Contact variables: {name}, {company}, {city}, {role}, {stack}, {company_description}, {legal_name}, {address}.").wrap());
                ui.add(egui::Label::new("Sender and technology: {sender_name}, {cms}, {crm}, {tech_stack}.").wrap());
                ui.add_space(8.0);
                ui.add(egui::Label::new("Use {Hello|Hi} to vary the wording. Each email receives one alternative.").wrap());
                ui.add(egui::Label::new(format!("Subject: {} variations. Message: {} variations.", analysis.subj_variants, analysis.body_variants)).wrap());
            });
        });
    }

    fn render_template_preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Email preview")
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE),
            );
        });
        ui.add_space(4.0);
        let mut flipper_changed = false;
        let contact = if !self.contacts.is_empty() {
            if self.preview_contact_index >= self.contacts.len() {
                self.preview_contact_index = 0;
            }
            self.contacts[self.preview_contact_index].clone()
        } else {
            self.selected_contact_id
                .as_ref()
                .and_then(|id| self.contacts.iter().find(|contact| &contact.id == id))
                .or_else(|| self.contacts.first())
                .cloned()
                .unwrap_or_else(|| {
                    ContactRecord::new(
                        "Alex Smith",
                        "info@concept-store.ru",
                        "Concept Store Krasnoyarsk",
                        "Founder / art director",
                        "Krasnoyarsk",
                        "Fashion / retail",
                        "+7 900 123-45-67",
                        "@concept_krsk",
                        "Demo",
                        "https://concept-store.ru",
                        LeadScore::Warm,
                        true,
                        "mx.yandex.ru.",
                    )
                })
        };
        if !self.contacts.is_empty() {
            let total = self.contacts.len();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Prospect:").small().color(TEXT_FAINT));
                if ui
                    .small_button("◄")
                    .on_hover_text("Preview previous lead in dataset")
                    .clicked()
                {
                    self.preview_contact_index = (self.preview_contact_index + total - 1) % total;
                    flipper_changed = true;
                }
                if ui
                    .small_button("►")
                    .on_hover_text("Preview next lead in dataset")
                    .clicked()
                {
                    self.preview_contact_index = (self.preview_contact_index + 1) % total;
                    flipper_changed = true;
                }
                let lead_desc = format!(
                    "{}/{} {} ({})",
                    self.preview_contact_index + 1,
                    total,
                    contact.name,
                    contact.company
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(lead_desc)
                            .small()
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .truncate(),
                );
            });
            ui.add_space(2.0);
        }
        ui.add(
            egui::Label::new(RichText::new(format!("Example for {}", contact.company)).weak())
                .wrap(),
        );
        let (refresh, shuffle) = ui
            .horizontal_wrapped(|ui| {
                let refresh = ui.button("Refresh preview").clicked();
                let shuffle = ui
                    .button("Shuffle variations")
                    .on_hover_text("Sample random spintax alternatives")
                    .clicked();
                if ui.button("Check template text").clicked() {
                    self.run_spam_check();
                }
                if ui.button("Generate 3 variants").clicked() {
                    self.generate_spintax_samples();
                }
                (refresh, shuffle)
            })
            .inner;
        if shuffle {
            self.show_toast("Sampled new spintax variation", true);
        }
        if flipper_changed {
            self.show_toast(format!("Previewing lead: {}", contact.name), true);
        }
        ui.add_space(10.0);
        self.editor_update_preview(&contact, refresh || shuffle || flipper_changed);
        let spam_report = crate::search::spam_trigger::check_spam_triggers(
            &self.template.subject,
            &self.template.body,
        );
        ui.horizontal_wrapped(|ui| {
            render_status_pill(
                ui,
                &spam_report.mailru_deliverability,
                Color32::from_rgb(74, 222, 128),
                Color32::from_rgb(5, 46, 22),
                Color32::from_rgb(22, 101, 52),
            );
            if spam_report.gmail_warning_reason.is_some() {
                render_status_pill(
                    ui,
                    &spam_report.gmail_risk_level,
                    Color32::from_rgb(251, 191, 36),
                    Color32::from_rgb(69, 26, 3),
                    Color32::from_rgb(154, 52, 18),
                );
            } else {
                render_status_pill(
                    ui,
                    &spam_report.gmail_risk_level,
                    Color32::from_rgb(74, 222, 128),
                    Color32::from_rgb(5, 46, 22),
                    Color32::from_rgb(22, 101, 52),
                );
            }
            if !spam_report.trigger_words_found.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "Триггеры: {}",
                        spam_report.trigger_words_found.join(", ")
                    ))
                    .small()
                    .color(TEXT_MUTED),
                );
            }
        });
        if let Some(reason) = &spam_report.gmail_warning_reason {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("ℹ ")
                        .color(Color32::from_rgb(251, 191, 36))
                        .strong(),
                );
                ui.label(RichText::new(reason).small().color(TEXT_MUTED));
            });
        }
        ui.add_space(8.0);
        card_frame(ui, |ui| {
            let from_addr = if !self.smtp.from_email.trim().is_empty() {
                self.smtp.from_email.as_str()
            } else {
                "sender@domain.ru"
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new("From:").small().monospace().color(TEXT_FAINT));
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{} <{}>", self.smtp.sender_name, from_addr))
                            .small()
                            .color(TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("To:  ").small().monospace().color(TEXT_FAINT));
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{} <{}>", contact.name, contact.email))
                            .small()
                            .color(TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Subj:").small().monospace().color(TEXT_FAINT));
                ui.add(
                    egui::Label::new(
                        RichText::new(&self.editor_preview_subject)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .wrap(),
                );
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(10.0);
            ui.add(egui::Label::new(&self.editor_preview_body).wrap());
        });
        if let Some(check) = &self.spam_check_result {
            ui.add_space(12.0);
            ui.collapsing(
                format!("Template text checks: {} / 100", check.score),
                |ui| {
                    if !check.has_personalization {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "Add contact personalization.",
                        );
                    }
                    if check.excessive_caps {
                        ui.colored_label(ui.visuals().warn_fg_color, "All caps detected.");
                    }
                    if check.excessive_punctuation {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "Repeated exclamation marks detected.",
                        );
                    }
                    if !check.trigger_words.is_empty() {
                        ui.add(
                            egui::Label::new(format!(
                                "Trigger words: {}",
                                check.trigger_words.join(", ")
                            ))
                            .wrap(),
                        );
                    }
                    for recommendation in &check.recommendations {
                        ui.add(egui::Label::new(recommendation).wrap());
                    }
                },
            );
        }
        if !self.spintax_samples.is_empty() {
            ui.add_space(12.0);
            ui.collapsing("Generated variations", |ui| {
                for (i, (subject, body)) in self.spintax_samples.iter().enumerate() {
                    ui.add(
                        egui::Label::new(RichText::new(format!("{}. {}", i + 1, subject)).strong())
                            .wrap(),
                    );
                    ui.add(egui::Label::new(body).wrap());
                    ui.add_space(12.0);
                }
            });
        }
    }

    fn render_template_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("template_workspace_scroll").show(ui, |ui| {
            workspace_heading(ui, "Email template", "Write once, personalize for each contact. Preview before sending.");
            ui.horizontal_wrapped(|ui| {
                if primary_button(ui, true, "Save template").clicked() {
                    self.save_template();
                }
                if ui.button("Load saved template").clicked() {
                    self.load_saved_template();
                }
                let mut preset_loaded = false;
                ui.menu_button("Built-in templates", |ui| {
                    if ui.button("Fashion and lookbook").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template = EmailTemplate {
                            template_name: "Fashion and production outreach".into(),
                            subject: "Visual content for {company}".into(),
                            body: "Hi {name},\n\nI create photography and visual content for brands. Would a short introduction about {company}'s next campaign be useful?\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid".into(),
                            ..EmailTemplate::default()
                        };
                        self.editor_update_preset_reply_address();
                    }
                    if ui.button("HoReCa and menus").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template.template_name = "HoReCa & Gastro Visuals".to_string();
                        self.template.subject = "{Menu photography|Food and interior photography|Visual content} for {company}".to_string();
                        self.template.body = "{Hello|Hi} {name},\n\nMy name is {sender_name}. I create visual content for restaurants and food brands in {city}: menu photography, interiors and short videos.\n\nAre you planning to update your menu visuals or social content this season?\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid".to_string();
                        self.editor_update_preset_reply_address();
                    }
                    if ui.button("Creative production / video").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template.template_name = "Creative Video & Production".to_string();
                        self.template.subject = "{Photo production|Video campaigns|Visual production} for {company}".to_string();
                        self.template.body = "{Hello|Hi} {name},\n\nI am {sender_name}. I create photography and video campaigns for brands and companies in {city}.\n\nWould you be open to collaborating on an upcoming campaign or production?\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid".to_string();
                        self.editor_update_preset_reply_address();
                    }
                    if ui.button("Follow-up #1").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template.template_name = "Follow-up #1".to_string();
                        self.template.subject = "Re: {Photography|Collaboration|Visual content} for {company}".to_string();
                        self.template.body = "{Hello|Hi} {name},\n\nJust following up to ask whether you had a chance to review my message\n\nHappy to share relevant examples or stay in touch for future projects.\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid".to_string();
                        self.editor_update_preset_reply_address();
                    }
                    if ui.button("B2B / founder").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template.template_name = "B2B Outreach".to_string();
                        self.template.subject = "Question about {stack} at {company}".to_string();
                        self.template.body = "Hi {name},\n\nI noticed your {stack} projects at {company} in {city}. I specialize in {stack} and help companies with production and delivery.\n\nWould extra support in this area be useful right now? I would be happy to share relevant portfolio examples.\n\nBest regards,\n{sender_name}".to_string();
                    }
                    if ui.button("IT / AppSec").clicked() {
                        preset_loaded = true;
                        ui.close();
                        self.template.template_name = "International Candidate Outreach (EN)".to_string();
                        self.template.subject = "Application / Security Collaboration — {company}".to_string();
                        self.template.body = "Hi {name},\n\n\
I hope you are having a productive week.\n\n\
I am reaching out regarding {role} and {stack} opportunities at {company}.\n\n\
My background centers on application security, vulnerability management, cloud infrastructure hardening, and security automation.\n\n\
I would welcome the opportunity to discuss how my skill set can support {company}'s current initiatives.\n\n\
Best regards,\n\
{sender_name}".to_string();
                    }
                });
                if preset_loaded {
                    self.template_notice = Some((true, "Built-in template loaded. Save template to keep it.".into()));
                    self.spam_check_result = None;
                    self.spintax_samples.clear();
                }
                ui.label(RichText::new(&self.template.template_name).weak());
            });
            if let Some((success, message)) = &self.template_notice {
                let color = if *success { success_color(ui) } else { ui.visuals().error_fg_color };
                ui.add(egui::Label::new(RichText::new(message).color(color)).wrap());
            } else {
                ui.add(egui::Label::new(RichText::new("Save keeps the current subject and message on this computer. Load saved restores your last save.").small().weak()).wrap());
            }
            ui.add_space(16.0);

            if ui.available_width() >= 820.0 {
                ui.columns(2, |columns| {
                    self.render_template_editor(&mut columns[0]);
                    self.render_template_preview(&mut columns[1]);
                });
            } else {
                self.render_template_editor(ui);
                ui.add_space(20.0);
                self.render_template_preview(ui);
            }
        });
    }

    fn editor_update_preview(&mut self, contact: &ContactRecord, refresh: bool) -> bool {
        let key = serde_json::to_string(&(
            &self.template.subject,
            &self.template.body,
            &self.smtp.sender_name,
            contact,
        ))
        .expect("Preview data contains only JSON-serializable contact and template fields");
        if !refresh && key == self.editor_preview_key {
            return false;
        }
        self.editor_preview_subject =
            mailer::render_template(&self.template.subject, contact, &self.smtp.sender_name);
        self.editor_preview_body =
            mailer::render_template(&self.template.body, contact, &self.smtp.sender_name);
        self.editor_preview_key = key;
        true
    }

    fn editor_update_preset_reply_address(&mut self) {
        let address = if self.smtp.reply_to.trim().is_empty() {
            self.smtp.from_email.trim()
        } else {
            self.smtp.reply_to.trim()
        };
        if !address.is_empty() {
            self.template.body = self.template.body.replace(
                "\nEmail: sender@example.invalid",
                &format!("\nEmail: {address}"),
            );
        }
    }

    fn editor_text_field(ui: &mut egui::Ui, label: &str, value: &mut String, password: bool) {
        ui.label(RichText::new(label).strong());
        ui.add(
            egui::TextEdit::singleline(value)
                .password(password)
                .desired_width(ui.available_width())
                .margin(egui::vec2(10.0, 8.0)),
        );
        ui.add_space(12.0);
    }

    fn editor_identity_panel(&mut self, ui: &mut egui::Ui) {
        card_frame(ui, |ui| {
            ui.label(RichText::new("Sender and replies").size(17.0).strong());
            ui.label(RichText::new("Choose how recipients see and reply to your email.").weak());
            ui.add_space(16.0);
            Self::editor_text_field(ui, "Sender name", &mut self.smtp.sender_name, false);
            Self::editor_text_field(
                ui,
                "Requested sender (From)",
                &mut self.smtp.from_email,
                false,
            );
            ui.label(
                RichText::new(if self.smtp.direct_mx {
                    "Used in the From header. Direct delivery does not verify ownership of this mailbox."
                } else {
                    "The provider must authorize this address. Editing From does not switch the sending account."
                })
                    .small()
                    .weak(),
            );
            ui.add_space(12.0);
            Self::editor_text_field(
                ui,
                "Receive replies at (Reply-To)",
                &mut self.smtp.reply_to,
                false,
            );
            ui.label(
                RichText::new("Replies go to this mailbox. Leave blank to use the sender email.")
                    .small()
                    .weak(),
            );
        });
    }

    fn editor_transport_panel(&mut self, ui: &mut egui::Ui) {
        card_frame(ui, |ui| {
            ui.label(RichText::new("Delivery connection").size(17.0).strong());
            ui.label(
                RichText::new(if self.smtp.direct_mx {
                    "Your computer connects straight to the recipient's mail server."
                } else {
                    "This computer connects to the selected mail service."
                })
                .weak(),
            );
            ui.add_space(16.0);
            egui::ComboBox::from_id_salt("editor_delivery_method")
                .selected_text(if self.smtp.direct_mx {
                    "Direct MX (advanced)"
                } else {
                    "SMTP email service"
                })
                .width(ui.available_width().min(280.0))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.smtp.direct_mx, false, "SMTP email service");
                    ui.selectable_value(&mut self.smtp.direct_mx, true, "Direct MX (advanced)");
                });
            ui.add_space(16.0);
            if self.smtp.direct_mx {
                ui.label("Connects directly to each recipient's mail server on port 25. Verified STARTTLS is required; no SMTP account is used.");
                ui.label(RichText::new("No email login or password is needed. The recipient may reject mail that fails sender authentication.").small().weak());
                if self
                    .smtp
                    .from_email
                    .trim()
                    .to_ascii_lowercase()
                    .ends_with("@vk.com")
                {
                    ui.add(egui::Label::new(RichText::new("VK sender authentication is not provided by Direct MX. Bulk inbox delivery is not verified.").color(ui.visuals().warn_fg_color)).wrap());
                }
                ui.add_space(12.0);
                ui.collapsing("DKIM signing", |ui| {
                    Self::editor_text_field(ui, "DNS selector", &mut self.smtp.dkim_selector, false);
                    Self::editor_text_field(ui, "Private key file (PEM)", &mut self.smtp.dkim_key_path, false);
                    ui.add(egui::Label::new(RichText::new("Publish the matching public key in your sender domain DNS. The private key stays on this computer; signing requires the installed OpenSSL executable.").small().weak()).wrap());
                });
                if ui.button("Check outbound port 25").clicked() {
                    self.test_port_25();
                }
                ui.add_space(12.0);
                ui.collapsing("Advanced: SMTP greeting", |ui| {
                    Self::editor_text_field(ui, "HELO / EHLO hostname", &mut self.smtp.helo_domain, false);
                    ui.label(RichText::new("Leave blank to use desktop.outreach.local. This is a computer hostname, not your email address.").small().weak());
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Provider presets:").small().color(TEXT_FAINT));
                    for (label, host, port, ssl) in [
                        ("Yandex 360", "smtp.yandex.ru", 465, true),
                        ("VK / Mail.ru", "smtp.mail.ru", 465, true),
                        ("Google Workspace", "smtp.gmail.com", 587, false),
                    ] {
                        let active = self.smtp.host == host
                            && self.smtp.port == port
                            && self.smtp.use_ssl == ssl;
                        if render_filter_chip(
                            ui,
                            active,
                            label,
                            &format!("Configure {label} ({host}:{port})"),
                        )
                        .clicked()
                        {
                            self.smtp.host = host.into();
                            self.smtp.port = port;
                            self.smtp.use_ssl = ssl;
                            self.show_toast(format!("Applied SMTP preset: {label}"), true);
                        }
                    }
                });
                ui.add_space(8.0);
                Self::editor_text_field(ui, "SMTP host", &mut self.smtp.host, false);
                ui.horizontal_wrapped(|ui| {
                    ui.label("Port");
                    ui.add(egui::DragValue::new(&mut self.smtp.port).range(1..=65535));
                    egui::ComboBox::from_id_salt("editor_smtp_encryption")
                        .selected_text(if self.smtp.use_ssl {
                            "SSL / TLS"
                        } else {
                            "STARTTLS"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.smtp.use_ssl, true, "SSL / TLS");
                            ui.selectable_value(&mut self.smtp.use_ssl, false, "STARTTLS");
                        });
                });
                ui.add_space(12.0);
                Self::editor_text_field(ui, "Login email", &mut self.smtp.username, false);
                ui.label(
                    RichText::new(
                        "This is the account authenticating to SMTP. It can appear as the sender.",
                    )
                    .small()
                    .weak(),
                );
                Self::editor_text_field(ui, "App password", &mut self.smtp.password, true);
                ui.label(
                    RichText::new(
                        "Use an app password from your email account's security settings.",
                    )
                    .small()
                    .weak(),
                );
            }
        });
    }

    fn render_smtp_tab(&mut self, ui: &mut egui::Ui) {
        workspace_heading(
            ui,
            "Mail and sending",
            if self.smtp.direct_mx {
                "Send directly from this computer. Set the From header and the mailbox for replies."
            } else {
                "Set the requested sender and replies. The connected provider controls the delivered sender."
            },
        );
        if primary_button(ui, true, "Save mail settings").clicked() {
            match save_app_settings(&self.smtp, &self.template, &self.settings_path) {
                Ok(()) => {
                    if let Err(error) =
                        mailer::validate_delivery_identity(&self.smtp, self.smtp.direct_mx)
                    {
                        self.status_message = format!("Settings saved. Sending blocked: {error}");
                        self.add_log(
                            "WARN",
                            "Mail settings saved; the sending identity is not connected.",
                        );
                    } else {
                        self.status_message =
                            "Mail settings saved. Confirm the sender in a received test email."
                                .into();
                        self.add_log(
                            "SUCCESS",
                            "Mail settings saved; recipient delivery is not yet verified.",
                        );
                    }
                }
                Err(error) => {
                    self.add_log("ERROR", &format!("Could not save mail settings: {error}"))
                }
            }
        }
        ui.add_space(16.0);
        egui::ScrollArea::vertical()
            .id_salt("mail_workspace_scroll")
            .show(ui, |ui| {
                let delivery_error =
                    mailer::validate_delivery_identity(&self.smtp, self.smtp.direct_mx).err();
                if let Some(error) = &delivery_error {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(26, 14, 16))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(127, 29, 29)))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠").strong().color(ACCENT_ROSE));
                                ui.label(
                                    RichText::new("Sending blocked").strong().color(ACCENT_ROSE),
                                );
                            });
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(error).color(Color32::from_rgb(244, 114, 182)),
                                )
                                .wrap(),
                            );
                            if self
                                .smtp
                                .from_email
                                .trim()
                                .to_ascii_lowercase()
                                .ends_with("@vk.com")
                            {
                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.hyperlink_to("Open VK Mail", "https://vk.mail.ru/");
                                    ui.hyperlink_to(
                                        "VK client support",
                                        "https://help.mail.ru/vkmail/faq/client/",
                                    );
                                    ui.hyperlink_to(
                                        "App password help",
                                        "https://help.mail.ru/vkmail/settings/2fa/login/",
                                    );
                                });
                            }
                        });
                    ui.add_space(16.0);
                }
                if ui.available_width() >= 760.0 {
                    ui.columns(2, |columns| {
                        self.editor_identity_panel(&mut columns[0]);
                        self.editor_transport_panel(&mut columns[1]);
                    });
                } else {
                    self.editor_identity_panel(ui);
                    ui.add_space(20.0);
                    self.editor_transport_panel(ui);
                }
                ui.add_space(16.0);
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match mailer::mail_identity(&self.smtp, self.smtp.direct_mx) {
                            Ok(identity) => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("●")
                                            .small()
                                            .color(Color32::from_rgb(52, 211, 153)),
                                    );
                                    ui.label(
                                        RichText::new(format!("Requested From: {}", identity.from))
                                            .small()
                                            .color(TEXT_MUTED),
                                    );
                                    ui.label(RichText::new("·").color(TEXT_FAINT));
                                    ui.label(
                                        RichText::new(format!("Replies: {}", identity.reply_to))
                                            .small()
                                            .color(Color32::from_rgb(96, 165, 250)),
                                    );
                                    if !self.smtp.direct_mx {
                                        ui.label(RichText::new("·").color(TEXT_FAINT));
                                        ui.label(
                                            RichText::new(format!(
                                                "Account: {}",
                                                self.smtp.username.trim()
                                            ))
                                            .small()
                                            .color(TEXT_FAINT),
                                        );
                                    }
                                });
                            }
                            Err(error) => {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("●").small().color(ACCENT_ROSE));
                                    ui.colored_label(ui.visuals().error_fg_color, error);
                                });
                            }
                        }
                    });
                ui.add_space(12.0);
                ui.add_space(20.0);
                card_frame(ui, |ui| {
                    ui.label(RichText::new("Send one test email").size(17.0).strong());
                    ui.label(
                        RichText::new("Uses the current template and the sender settings above.")
                            .weak(),
                    );
                    ui.add_space(12.0);
                    Self::editor_text_field(
                        ui,
                        "Test recipient email",
                        &mut self.test_email_target,
                        false,
                    );
                    if ui
                        .add_enabled(
                            !self.is_mailing
                                && delivery_error.is_none()
                                && !self.test_email_target.trim().is_empty(),
                            egui::Button::new("Send test email"),
                        )
                        .clicked()
                    {
                        self.send_test_email();
                    }
                });
                ui.add_space(20.0);
                ui.collapsing("Sending pace and unsubscribe", |ui| {
                    ui.add_space(8.0);
                    ui.label("Delay between emails");
                    ui.add(egui::Slider::new(&mut self.smtp.delay_seconds, 1..=60).text("seconds"));
                    ui.checkbox(&mut self.smtp.random_delay, "Vary the delay (70%–140%)");
                    ui.add_space(12.0);
                    ui.label("Daily sending limit");
                    ui.add(egui::Slider::new(&mut self.smtp.daily_limit, 10..=300).text("emails"));
                    ui.label(
                        RichText::new(format!(
                            "Sent today: {} / {}",
                            self.smtp.sent_today, self.smtp.daily_limit
                        ))
                        .weak(),
                    );
                    ui.checkbox(
                        &mut self.smtp.warmup_mode,
                        "Gradual warmup (+5 emails per day)",
                    );
                    ui.add_space(12.0);
                    ui.checkbox(
                        &mut self.smtp.enable_rfc8058,
                        "Include an email unsubscribe link in the headers",
                    );
                    ui.checkbox(
                        &mut self.smtp.request_read_receipt,
                        "Request delivery/read receipt (Disposition-Notification-To)",
                    );
                });
                ui.add_space(12.0);
                ui.collapsing("Sender domain diagnostics", |ui| {
                    ui.add_space(8.0);
                    Self::editor_text_field(
                        ui,
                        "Sender domain",
                        &mut self.audit_domain_input,
                        false,
                    );
                    let label = if self.is_auditing_sender {
                        "Auditing..."
                    } else {
                        "Run domain audit"
                    };
                    if ui
                        .add_enabled(!self.is_auditing_sender, egui::Button::new(label))
                        .clicked()
                    {
                        self.start_sender_audit();
                    }
                    if let Some((audit, dnsbl)) = &self.sender_audit_result {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(format!(
                                "{} · Grade {} · {}%",
                                audit.domain, audit.grade, audit.score
                            ))
                            .strong(),
                        );
                        for (label, found) in [
                            ("MX", audit.has_mx),
                            ("SPF", audit.has_spf),
                            ("DMARC", audit.has_dmarc),
                            ("DKIM", audit.has_dkim),
                            ("MTA-STS", audit.has_mta_sts),
                            ("TLS-RPT", audit.has_tls_rpt),
                            ("BIMI", audit.has_bimi),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(label);
                                ui.colored_label(
                                    if found {
                                        success_color(ui)
                                    } else {
                                        ui.visuals().warn_fg_color
                                    },
                                    if found { "Found" } else { "Not found" },
                                );
                            });
                        }
                        ui.label(format!(
                            "DNSBL: {}",
                            if dnsbl.listed { "Listed" } else { "Clear" }
                        ));
                        if dnsbl.listed {
                            ui.label(format!("{} detections", dnsbl.detections.len()));
                        }
                        ui.collapsing("DNS record details", |ui| {
                            ui.label(format!("MX: {}", audit.mx_record));
                            ui.label(format!("SPF: {}", audit.spf_record));
                            ui.label(format!("DMARC: {}", audit.dmarc_record));
                            ui.label(format!(
                                "DKIM: {} ({} bits)",
                                audit.dkim_selector, audit.dkim_bits
                            ));
                            ui.label(format!("MTA-STS: {}", audit.mta_sts_mode));
                            if !audit.bimi_vmc_cert.is_empty() {
                                ui.label(format!("BIMI VMC: {}", audit.bimi_vmc_cert));
                            }
                        });
                        for recommendation in &audit.recommendations {
                            ui.label(recommendation);
                        }
                    }
                });
                ui.add_space(12.0);
                ui.collapsing("Send to selected contacts", |ui| {
                    ui.label(
                        RichText::new("These actions use the recipients selected in Contacts.")
                            .weak(),
                    );
                    ui.add_space(8.0);
                    self.mailing_actions(ui);
                });
            });
    }

    fn render_logs_tab(&mut self, ui: &mut egui::Ui) {
        workspace_heading(
            ui,
            "Activity log",
            "Collection progress, saved results and delivery events.",
        );
        let total_count = self.logs.len();
        let err_count = self.logs.iter().filter(|l| l.level == "ERROR").count();
        let warn_count = self.logs.iter().filter(|l| l.level == "WARN").count();
        let ok_count = self.logs.iter().filter(|l| l.level == "SUCCESS").count();

        ui.horizontal_wrapped(|ui| {
            if self.is_searching && ui.button("Stop collection").clicked() {
                self.collection_cancel.store(true, Ordering::Relaxed);
                self.status_message =
                    "Stopping collection. Collected contacts remain saved.".into();
            }

            ui.label(
                RichText::new(format!("{} events", total_count))
                    .small()
                    .color(TEXT_MUTED),
            );
            if err_count > 0 {
                ui.label(
                    RichText::new(format!("· {} errors", err_count))
                        .small()
                        .color(ACCENT_ROSE),
                );
            }
            if warn_count > 0 {
                ui.label(
                    RichText::new(format!("· {} warnings", warn_count))
                        .small()
                        .color(ACCENT_AMBER),
                );
            }
            if ok_count > 0 {
                ui.label(
                    RichText::new(format!("· {} ok", ok_count))
                        .small()
                        .color(ACCENT_TEAL),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear log").clicked() {
                    self.logs.clear();
                    self.show_toast("Cleared activity logs", true);
                }
                if ui
                    .button("Copy logs")
                    .on_hover_text("Copy filtered logs to clipboard")
                    .clicked()
                {
                    let query = self.log_search_query.trim().to_lowercase();
                    let filtered: Vec<_> = self
                        .logs
                        .iter()
                        .filter(|l| {
                            let matches_level = match self.log_level_filter.as_str() {
                                "Errors" => l.level == "ERROR",
                                "Warnings" => l.level == "WARN",
                                "Info" => l.level == "INFO" || l.level == "SUCCESS",
                                _ => true,
                            };
                            let matches_query = query.is_empty()
                                || l.message.to_lowercase().contains(&query)
                                || l.level.to_lowercase().contains(&query);
                            matches_level && matches_query
                        })
                        .map(|l| format!("[{}] {} {}", l.time, l.level, l.message))
                        .collect();
                    ui.ctx().copy_text(filtered.join("\n"));
                    self.status_message = format!("Copied {} log entries.", filtered.len());
                    self.show_toast(format!("Copied {} log entries", filtered.len()), true);
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (level, count) in [
                ("All", total_count),
                ("Errors", err_count),
                ("Warnings", warn_count),
                ("Info", total_count.saturating_sub(err_count + warn_count)),
            ] {
                let active = self.log_level_filter == level;
                let label = format!("{level} · {count}");
                if render_filter_chip(ui, active, &label, &format!("Filter logs: {level}"))
                    .clicked()
                {
                    self.log_level_filter = level.to_string();
                }
            }
            ui.add_space(8.0);
            let scroll_active = self.log_autoscroll;
            let scroll_label = if scroll_active {
                "Scroll: Auto"
            } else {
                "Scroll: Paused"
            };
            let scroll_tip = if scroll_active {
                "Auto-scrolling to newest events. Click to pause."
            } else {
                "Scroll locked. Click to resume auto-scroll."
            };
            if render_filter_chip(ui, scroll_active, scroll_label, scroll_tip).clicked() {
                self.log_autoscroll = !self.log_autoscroll;
                self.show_toast(
                    if self.log_autoscroll {
                        "Auto-scroll resumed"
                    } else {
                        "Auto-scroll paused"
                    },
                    true,
                );
            }
            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.log_search_query)
                                .frame(egui::Frame::NONE)
                                .desired_width(170.0)
                                .hint_text("Filter logs…"),
                        );
                        if !self.log_search_query.is_empty() {
                            if ui
                                .small_button(RichText::new("×").size(10.0).color(TEXT_MUTED))
                                .on_hover_text("Clear log filter")
                                .clicked()
                            {
                                self.log_search_query.clear();
                            }
                        }
                    });
                });
        });
        ui.add_space(12.0);
        if self.is_searching && self.automatic_business_collection {
            let progress = (self.search_collected_count as f32
                / self.filter.max_items.max(1) as f32)
                .clamp(0.0, 1.0);
            let pct = (progress * 100.0) as u32;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} / {} companies saved",
                        self.search_collected_count, self.filter.max_items
                    ))
                    .strong()
                    .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{pct}%"))
                            .strong()
                            .color(Color32::from_rgb(52, 211, 153)),
                    );
                });
            });
            ui.add_space(4.0);
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_height(6.0)
                    .corner_radius(egui::CornerRadius::same(3))
                    .fill(Color32::WHITE),
            );
            ui.add_space(14.0);
        }
        card_frame(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("activity_events")
                .stick_to_bottom(self.log_autoscroll)
                .show(ui, |ui| {
                    let query = self.log_search_query.trim().to_lowercase();
                    let filtered_logs: Vec<_> = self
                        .logs
                        .iter()
                        .filter(|l| {
                            let matches_level = match self.log_level_filter.as_str() {
                                "Errors" => l.level == "ERROR",
                                "Warnings" => l.level == "WARN",
                                "Info" => l.level == "INFO" || l.level == "SUCCESS",
                                _ => true,
                            };
                            let matches_query = query.is_empty()
                                || l.message.to_lowercase().contains(&query)
                                || l.level.to_lowercase().contains(&query);
                            matches_level && matches_query
                        })
                        .collect();
                    if filtered_logs.is_empty() {
                        ui.label(
                            RichText::new(if self.logs.is_empty() {
                                "No events yet. Start a collection to see progress here."
                            } else {
                                "No events matching the active filter."
                            })
                            .weak(),
                        );
                    }
                    for log in filtered_logs {
                        ui.horizontal_top(|ui| {
                            ui.add_sized(
                                [60.0, 18.0],
                                egui::Label::new(
                                    RichText::new(&log.time)
                                        .monospace()
                                        .small()
                                        .color(TEXT_FAINT),
                                ),
                            );
                            let (level_text, level_color) = match log.level.as_str() {
                                "ERROR" => ("ERROR", ACCENT_ROSE),
                                "WARN" => ("WARN", ACCENT_AMBER),
                                "SUCCESS" => ("OK", ACCENT_TEAL),
                                _ => ("INFO", TEXT_MUTED),
                            };
                            ui.add_sized(
                                [46.0, 18.0],
                                egui::Label::new(
                                    RichText::new(level_text)
                                        .monospace()
                                        .small()
                                        .color(level_color)
                                        .strong(),
                                ),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(compact_log_message(&log.message))
                                        .size(13.0)
                                        .color(Color32::from_rgb(226, 232, 240)),
                                )
                                .wrap(),
                            )
                            .on_hover_text(&log.message);
                        });
                        ui.add_space(6.0);
                    }
                });
        });
    }
}

fn configure_ui(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("Segoe UI".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Segoe UI".into());

        if let Ok(sym_bytes) = std::fs::read("C:/Windows/Fonts/seguisym.ttf") {
            fonts.font_data.insert(
                "Segoe UI Symbol".into(),
                egui::FontData::from_owned(sym_bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("Segoe UI Symbol".into());
        }

        ctx.set_fonts(fonts);
    }
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_COLOR;
    visuals.window_fill = CARD_BG;
    visuals.extreme_bg_color = TEXT_DARK;
    visuals.text_edit_bg_color = Some(TEXT_DARK);
    visuals.faint_bg_color = Color32::from_rgb(15, 15, 18);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals.override_text_color = None;
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.selection.bg_fill = Color32::from_rgb(38, 38, 44);
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = ACCENT_CYAN;
    visuals.error_fg_color = ACCENT_ROSE;
    visuals.warn_fg_color = ACCENT_AMBER;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_COLOR);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);

    // Inactive buttons and inputs: clean neutral dark surface with 1px border
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_COLOR);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 18, 22);
    visuals.widgets.inactive.weak_bg_fill = visuals.widgets.inactive.bg_fill;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, CONTROL_BORDER);

    // Hovered widgets: crisp neutral hover
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 28, 34);
    visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, TEXT_MUTED);

    // Active widgets:
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.bg_fill = Color32::from_rgb(36, 36, 42);
    visuals.widgets.active.weak_bg_fill = visuals.widgets.active.bg_fill;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Open menus:
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.open.bg_fill = Color32::from_rgb(28, 28, 34);
    visuals.widgets.open.weak_bg_fill = visuals.widgets.open.bg_fill;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, CONTROL_BORDER);

    ctx.set_visuals_of(egui::Theme::Dark, visuals);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.interact_size = Vec2::new(32.0, CONTROL_HEIGHT);
        style.animation_time = 0.0;
        for (text, size) in [
            (egui::TextStyle::Heading, 20.0),
            (egui::TextStyle::Body, 13.5),
            (egui::TextStyle::Button, 13.5),
            (egui::TextStyle::Small, 11.5),
            (egui::TextStyle::Monospace, 12.0),
        ] {
            let family = if text == egui::TextStyle::Monospace {
                egui::FontFamily::Monospace
            } else {
                egui::FontFamily::Proportional
            };
            style
                .text_styles
                .insert(text, egui::FontId::new(size, family));
        }
    });
}

fn workspace_heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new(title).strong().color(Color32::WHITE));
    });
    ui.add_space(2.0);
    ui.add(egui::Label::new(RichText::new(description).color(TEXT_MUTED)).wrap());
    ui.add_space(14.0);
}

fn compact_log_message(message: &str) -> String {
    message
        .split_whitespace()
        .map(|word| {
            if word.chars().count() > 100 && word.contains("http") {
                let clean = word.trim_start_matches('(');
                if let Ok(url) = url::Url::parse(clean) {
                    return format!(
                        "{}://{}/…",
                        url.scheme(),
                        url.host_str().unwrap_or("source")
                    );
                }
                return format!("{}…", word.chars().take(76).collect::<String>());
            }
            word.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn success_color(_ui: &egui::Ui) -> Color32 {
    ACCENT_TEAL
}

fn primary_button(
    ui: &mut egui::Ui,
    enabled: bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let text = if enabled {
        label.into().color(Color32::from_rgb(9, 9, 11)).strong()
    } else {
        label.into().color(TEXT_FAINT)
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .fill(if enabled {
                Color32::WHITE
            } else {
                Color32::from_rgb(24, 24, 27)
            })
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(Vec2::new(0.0, CONTROL_HEIGHT)),
    )
}

fn render_filter_chip(
    ui: &mut egui::Ui,
    active: bool,
    label: &str,
    tooltip: &str,
) -> egui::Response {
    let text = egui::RichText::new(label).size(12.5).color(if active {
        Color32::WHITE
    } else {
        TEXT_MUTED
    });
    let stroke = Stroke::new(
        1.0,
        if active {
            Color32::from_rgb(82, 82, 91)
        } else {
            BORDER_COLOR
        },
    );
    let bg = if active {
        Color32::from_rgb(32, 32, 38)
    } else {
        CARD_BG
    };

    let button = egui::Button::new(text)
        .fill(bg)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .min_size(Vec2::new(0.0, CONTROL_HEIGHT));

    ui.add(button).on_hover_text(tooltip)
}

fn render_status_pill(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32, border: Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(fg));
        });
}

fn render_kbd(ui: &mut egui::Ui, text: &str) {
    let padding = egui::vec2(5.0, 2.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(10.5),
        Color32::from_rgb(228, 228, 231),
    );
    let size = egui::vec2((galley.size().x + padding.x * 2.0).max(20.0), 18.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 4.0, Color32::from_rgb(26, 26, 32));
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::from_rgb(63, 63, 70)),
        egui::StrokeKind::Inside,
    );
    let top_y = rect.min.y + 1.0;
    ui.painter().line_segment(
        [
            egui::pos2(rect.min.x + 2.0, top_y),
            egui::pos2(rect.max.x - 2.0, top_y),
        ],
        Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 28)),
    );
    let text_pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter()
        .galley(text_pos, galley, Color32::from_rgb(228, 228, 231));
}

fn table_cell(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 48.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
}

fn card_frame<R>(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(18.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // ui.columns() hands cards a justified layout, which stretches the word gaps
            // of every wrapped label; cards lay out top-down without that.
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), add)
                .inner
        })
}

fn poisk_app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    let s = size as f32;
    let center = s / 2.0;

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let idx = (y * size + x) * 4;

            let margin = 3.0;
            let radius = 12.0;
            let min = margin;
            let max = s - margin;
            let dx = if px < min + radius {
                (min + radius) - px
            } else if px > max - radius {
                px - (max - radius)
            } else {
                0.0
            };
            let dy = if py < min + radius {
                (min + radius) - py
            } else if py > max - radius {
                py - (max - radius)
            } else {
                0.0
            };
            let dist_corner = (dx * dx + dy * dy).sqrt();

            if px >= min && px <= max && py >= min && py <= max && dist_corner <= radius {
                let mut r = 20u8;
                let mut g = 20u8;
                let mut b = 24u8;
                let a = 255u8;

                let border_dist = radius - dist_corner;
                let edge_dist = (px - min).min(max - px).min(py - min).min(max - py);
                if (dist_corner > 0.0 && border_dist < 2.0)
                    || (dist_corner == 0.0 && edge_dist < 2.0)
                {
                    r = 39;
                    g = 39;
                    b = 42;
                }

                let c_dx = px - center;
                let c_dy = py - center;
                let c_dist = (c_dx * c_dx + c_dy * c_dy).sqrt();

                if (c_dist - 14.5).abs() < 1.6 {
                    r = 161;
                    g = 161;
                    b = 170;
                }

                if (py - center).abs() < 1.4 && px >= center - 19.0 && px <= center - 11.0 {
                    r = 161;
                    g = 161;
                    b = 170;
                }
                if (py - center).abs() < 1.4 && px >= center + 11.0 && px <= center + 19.0 {
                    r = 161;
                    g = 161;
                    b = 170;
                }
                if (px - center).abs() < 1.4 && py >= center + 11.0 && py <= center + 19.0 {
                    r = 161;
                    g = 161;
                    b = 170;
                }

                if c_dx >= 0.0 && c_dy <= 0.0 && (c_dx + c_dy).abs() < 1.8 && c_dist <= 18.0 {
                    r = 255;
                    g = 255;
                    b = 255;
                }

                let spark_x = center + 12.5;
                let spark_y = center - 12.5;
                let s_dx = px - spark_x;
                let s_dy = py - spark_y;
                let s_dist = (s_dx * s_dx + s_dy * s_dy).sqrt();
                if s_dist < 4.2 {
                    r = 255;
                    g = 255;
                    b = 255;
                }

                if c_dist < 4.5 {
                    r = 228;
                    g = 228;
                    b = 231;
                }

                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = a;
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

fn main() -> eframe::Result {
    #[cfg(debug_assertions)]
    let compact_review =
        std::env::var("POISK_UI_REVIEW").is_ok_and(|review| review.contains("compact"));
    #[cfg(not(debug_assertions))]
    let compact_review = false;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PoiskContact — Contact search and outreach")
            .with_icon(poisk_app_icon())
            .with_inner_size(if compact_review {
                [900.0, 600.0]
            } else {
                [1180.0, 780.0]
            })
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PoiskContact",
        options,
        Box::new(|cc| Ok(Box::new(AppState::new(cc)))),
    )
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    #[test]
    fn template_buttons_save_reload_and_display_errors_without_losing_edits() {
        fn click(ctx: &egui::Context, app: &mut AppState, label: &str) -> Vec<String> {
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1180.0, 780.0),
                )),
                ..Default::default()
            };
            let mut position = None;
            for _ in 0..4 {
                let mut output = ctx.run_ui(input(), |ui| app.render_workspace(ui));
                position = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                });
                output.textures_delta.clear();
            }
            let position = position.expect("The template action must be visible");
            for pressed in [true, false] {
                let mut input = input();
                input.events = vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                let mut output = ctx.run_ui(input, |ui| app.render_workspace(ui));
                output.textures_delta.clear();
            }
            let mut output = ctx.run_ui(input(), |ui| app.render_workspace(ui));
            let visible = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
            visible
        }
        let directory = isolated_directory("template-buttons");
        let mut app = AppState::load(&directory);
        let ctx = egui::Context::default();
        configure_ui(&ctx);
        app.active_tab = ActiveTab::Template;
        app.template.subject = "Съёмка для {company}".into();
        app.template.body = "Мой собственный текст\nВторая строка".repeat(100);
        let saved_subject = app.template.subject.clone();
        let saved_body = app.template.body.clone();
        let visible = click(&ctx, &mut app, "Save template");
        assert!(
            visible
                .iter()
                .any(|text| text.contains("Template saved locally"))
        );
        assert!(app.template_notice.as_ref().unwrap().0);
        let restarted = AppState::load(&directory);
        assert_eq!(restarted.template.subject, saved_subject);
        assert_eq!(restarted.template.body, saved_body);
        app.template.subject = "Temporary edit".into();
        app.template.body.clear();
        app.smtp.username = "keep-unsaved-login@example.org".into();
        let visible = click(&ctx, &mut app, "Load saved template");
        assert!(
            visible
                .iter()
                .any(|text| text.contains("Saved template loaded"))
        );
        assert_eq!(app.template.subject, saved_subject);
        assert_eq!(app.template.body, saved_body);
        assert_eq!(app.smtp.username, "keep-unsaved-login@example.org");
        assert!(app.template_notice.as_ref().unwrap().1.contains("loaded"));
        let visible = click(&ctx, &mut app, "Built-in templates");
        assert!(visible.iter().any(|text| text == "Fashion and lookbook"));
        click(&ctx, &mut app, "HoReCa and menus");
        assert_eq!(app.template.template_name, "HoReCa & Gastro Visuals");
        assert_eq!(
            storage::load_email_template(&app.settings_path)
                .unwrap()
                .body,
            saved_body
        );
        click(&ctx, &mut app, "Load saved template");
        assert_eq!(app.template.body, saved_body);
        std::fs::write(&app.settings_path, b"broken JSON").unwrap();
        app.load_saved_template();
        assert!(!app.template_notice.as_ref().unwrap().0);
        assert_eq!(app.template.body, saved_body);
        app.save_template();
        assert!(!app.template_notice.as_ref().unwrap().0);
        assert_eq!(
            std::fs::read_to_string(&app.settings_path).unwrap(),
            "broken JSON"
        );
        app.settings_path = directory.join("missing/settings.json");
        app.save_template();
        assert!(!app.template_notice.as_ref().unwrap().0);
    }

    fn isolated_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "poisk-{label}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn template_preview_stays_stable_until_its_inputs_or_refresh_change() {
        let directory = isolated_directory("stable-template-preview");
        let mut app = AppState::load(&directory);
        app.template.subject = "{Hello|Hi} {company}".into();
        app.template.body = "{Welcome|Greetings} {name}, from {sender_name}".into();
        app.smtp.sender_name = "Sender one".into();
        let mut contact = ContactRecord {
            company: "Example company".into(),
            name: "Alex".into(),
            ..Default::default()
        };
        assert!(app.editor_update_preview(&contact, false));
        let preview = (
            app.editor_preview_subject.clone(),
            app.editor_preview_body.clone(),
        );
        for _ in 0..100 {
            assert!(!app.editor_update_preview(&contact, false));
            assert_eq!(
                (&app.editor_preview_subject, &app.editor_preview_body),
                (&preview.0, &preview.1)
            );
        }
        assert!(app.editor_update_preview(&contact, true));
        app.smtp.sender_name = "Sender two".into();
        assert!(app.editor_update_preview(&contact, false));
        assert!(app.editor_preview_body.contains("Sender two"));
        contact.company = "Updated company".into();
        assert!(app.editor_update_preview(&contact, false));
        assert!(app.editor_preview_subject.contains("Updated company"));
        app.template.subject = "Changed subject".into();
        assert!(app.editor_update_preview(&contact, false));
        assert_eq!(app.editor_preview_subject, "Changed subject");
        app.template.body = "Changed message".into();
        assert!(app.editor_update_preview(&contact, false));
        assert_eq!(app.editor_preview_body, "Changed message");
        assert!(!directory.join("settings.json").exists());
        drop(app);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn chrome_crawler_import_merges_with_backup_and_preserves_history() {
        let directory = isolated_directory("chrome-crawler-import");
        let original = ContactRecord {
            id: "existing-company".into(),
            company: "Studio".into(),
            email: "office@example.org".into(),
            selected: true,
            status: SendStatus::Sent,
            follow_up_count: 2,
            last_message_id: Some("<old@sender.org>".into()),
            last_sent_at: Some("2026-10-17 12:00:00".into()),
            notes: "Keep my note".into(),
            ..Default::default()
        };
        save_contacts_to_json(&[original], directory.join("contacts.json")).unwrap();
        let before = std::fs::read(directory.join("contacts.json")).unwrap();
        let mut app = AppState::load(&directory);
        app.chrome_crawler_input = serde_json::json!({
            "format": "poisk-contact.chrome-crawler.v1", "contacts": [
                {"company": "Studio", "email": "office@example.org", "phone": "+7 391 1234567",
                 "source_url": "https://example.org/contact", "evidence_text": "office@example.org +7 391 1234567"},
                {"company": "Second studio", "email": "hello@second.example",
                 "source_url": "https://second.example/contact", "evidence_text": "hello@second.example"}
            ]
        }).to_string();
        app.import_chrome_crawler();
        assert!(app.chrome_crawler_input.is_empty());
        let records = load_contacts_from_json(directory.join("contacts.json")).unwrap();
        assert_eq!(records.len(), 2);
        let existing = records.iter().find(|r| r.id == "existing-company").unwrap();
        assert!(existing.selected);
        assert_eq!(existing.status, SendStatus::Sent);
        assert_eq!(existing.follow_up_count, 2);
        assert_eq!(
            existing.last_message_id.as_deref(),
            Some("<old@sender.org>")
        );
        assert!(existing.notes.contains("Keep my note"));
        assert_eq!(existing.phone, "+7 (391) 123-45-67");
        let new = records
            .iter()
            .find(|r| r.company == "Second studio")
            .unwrap();
        assert!(!new.selected && !new.mx_valid && !new.rcpt_verified);
        assert_eq!(new.evidence.len(), 1);
        let backup = std::fs::read_dir(directory.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(std::fs::read(backup).unwrap(), before);
        assert!(directory.join("contacts.csv").exists());
    }

    #[test]
    fn chrome_crawler_invalid_input_busy_state_and_corrupt_database_do_not_write() {
        let directory = isolated_directory("chrome-crawler-invalid");
        let mut app = AppState::load(&directory);
        app.chrome_crawler_input = "{broken".into();
        app.import_chrome_crawler();
        assert!(app.contacts.is_empty() && !app.chrome_crawler_input.is_empty());
        assert!(!directory.join("contacts.json").exists());
        for mailing in [true, false] {
            app.is_mailing = mailing;
            app.is_searching = !mailing;
            app.status_message = "busy".into();
            app.import_chrome_crawler();
            assert_eq!(app.status_message, "busy");
        }
        std::fs::write(directory.join("contacts.json"), "{invalid database").unwrap();
        let mut app = AppState::load(&directory);
        app.chrome_crawler_input = "{}".into();
        app.import_chrome_crawler();
        assert!(app.status_message.contains("Import blocked"));
        assert_eq!(
            std::fs::read_to_string(directory.join("contacts.json")).unwrap(),
            "{invalid database"
        );
    }

    #[test]
    fn hidden_window_logic_saves_collection_without_a_render_pass() {
        let directory = isolated_directory("hidden-window-save");
        let mut app = AppState::load(&directory);
        let (tx, rx) = channel();
        app.search_rx = Some(rx);
        app.is_searching = true;
        tx.send(SearchProgress::Collected(vec![ContactRecord {
            id: "background-company".into(),
            company: "Background company".into(),
            url: "https://background.example/".into(),
            email: "office@background.example".into(),
            ..Default::default()
        }]))
        .unwrap();
        let ctx = egui::Context::default();
        let mut frame = eframe::Frame::_new_kittest();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .minimized = Some(true);
        let _ = ctx.run_logic(&input, |ctx| eframe::App::logic(&mut app, ctx, &mut frame));
        let saved = load_contacts_from_json(directory.join("contacts.json")).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, "background-company");
        assert!(directory.join("contacts.csv").exists());
        assert!(app.is_searching);
    }

    #[test]
    fn incoming_collection_is_saved_before_completion_with_history_and_backup() {
        let directory = isolated_directory("incremental-save");
        let mut existing = ContactRecord {
            id: "existing-id".into(),
            company: "Existing company".into(),
            city: "Krasnoyarsk".into(),
            url: "https://business.org/".into(),
            email: "office@business.org".into(),
            status: SendStatus::Sent,
            notes: "Keep my note".into(),
            last_message_id: Some("<old@sender.org>".into()),
            last_sent_at: Some("2026-10-17 12:00:00".into()),
            follow_up_count: 2,
            ..Default::default()
        };
        save_contacts_to_json(&[existing.clone()], directory.join("contacts.json")).unwrap();
        let original = std::fs::read(directory.join("contacts.json")).unwrap();
        let mut app = AppState::load(&directory);
        let mut update = existing.clone();
        update.id = "discovered-id".into();
        update.lead_score = 99;
        update.status = SendStatus::Pending;
        update.notes = "Published website proof".into();
        update.follow_up_count = 0;
        update.last_message_id = None;
        update.last_sent_at = None;
        existing.id = "new-business".into();
        existing.url = "https://second-business.org/".into();
        existing.email = "office@second-business.org".into();
        existing.company = "Second business".into();
        existing.status = SendStatus::Pending;
        let (tx, rx) = channel();
        app.search_rx = Some(rx);
        app.is_searching = true;
        tx.send(SearchProgress::Collected(vec![update, existing]))
            .unwrap();
        app.poll_async_events();
        let saved = load_contacts_from_json(directory.join("contacts.json")).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(app.is_searching);
        let updated = saved.iter().find(|r| r.id == "existing-id").unwrap();
        assert_eq!(updated.status, SendStatus::Sent);
        assert_eq!(updated.follow_up_count, 2);
        assert_eq!(updated.last_message_id.as_deref(), Some("<old@sender.org>"));
        assert!(updated.notes.contains("Keep my note"));
        let backup = std::fs::read_dir(directory.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(std::fs::read(backup).unwrap(), original);
        tx.send(SearchProgress::Error(
            "Provider unavailable; partial contacts retained".into(),
        ))
        .unwrap();
        app.poll_async_events();
        assert!(!app.is_searching);
        assert_eq!(AppState::load(&directory).contacts.len(), 2);
        assert!(directory.join("contacts.csv").exists());
    }

    #[test]
    fn corrupt_database_is_preserved_when_starting_collection() {
        let directory = isolated_directory("invalid-database");
        let path = directory.join("contacts.json");
        std::fs::write(&path, b"{invalid-json").unwrap();
        let mut app = AppState::load(&directory);
        app.start_search();
        assert!(!app.is_searching);
        assert!(app.database_load_error.is_some());
        assert_eq!(std::fs::read(path).unwrap(), b"{invalid-json");
    }

    #[test]
    #[ignore = "Uses live public search and company websites; saves only to an isolated temporary database"]
    fn live_automatic_collection_saves_to_isolated_database() {
        let directory = isolated_directory("live-gui-collection");
        save_contacts_to_json(&[], directory.join("contacts.json")).unwrap();
        let mut app = AppState::load(&directory);
        app.filter.city = "Красноярск".into();
        app.filter.max_items = 3;
        app.filter.verify_mx = false;
        app.automatic_business_collection = true;
        app.all_industries = true;
        app.start_search();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while app.is_searching && std::time::Instant::now() < deadline {
            app.poll_async_events();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if app.is_searching {
            app.collection_cancel.store(true, Ordering::Relaxed);
            panic!(
                "Live collection exceeded the test budget; partial data remains at {}",
                directory.display()
            );
        }
        let records = load_contacts_from_json(directory.join("contacts.json")).unwrap();
        for entry in &app.logs {
            println!("{}: {}", entry.level, entry.message);
        }
        assert!(
            !records.is_empty(),
            "Live providers yielded no verified contacts; see logs"
        );
        assert!(records.len() <= 3);
        assert!(
            records
                .iter()
                .all(|record| !record.selected && !record.evidence.is_empty())
        );
        println!(
            "LIVE GUI COLLECTION: {} published company records saved through start_search / poll_async_events; database: {}",
            records.len(),
            directory.display()
        );
    }

    fn test_app() -> AppState {
        let directory = std::env::temp_dir().join(format!(
            "poisk-ui-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let mut app = AppState::load(&directory);
        for n in 0..15 {
            app.contacts.push(ContactRecord::new(
                "Test contact",
                &format!("contact{n}@example.org"),
                "Company with a very long name to check table width",
                "Head of engineering / Senior Full-stack Developer (Remote worldwide)",
                "Krasnoyarsk",
                "AppSec",
                &format!("+7 (391) 234-00-{n:02}"),
                "",
                "Test",
                &format!("https://company{n}.example.org"),
                LeadScore::Hot,
                true,
                "mx.example.org",
            ));
        }
        app
    }

    #[test]
    fn workspace_stays_inside_viewport() {
        let mut app = test_app();
        app.filter.city = "Улан-Удэ".into();
        app.filter.stack = "мебельные фабрики".into();
        app.filter.max_items = 1_000;
        for theme in [egui::ThemePreference::Dark] {
            for size in [
                Vec2::new(1180.0, 780.0),
                Vec2::new(900.0, 600.0),
                Vec2::new(600.0, 400.0),
            ] {
                for tab in [
                    ActiveTab::Search,
                    ActiveTab::Contacts,
                    ActiveTab::Template,
                    ActiveTab::Smtp,
                    ActiveTab::Logs,
                ] {
                    let ctx = egui::Context::default();
                    configure_ui(&ctx);
                    ctx.set_theme(theme);
                    app.active_tab = tab;
                    let mut used = egui::Rect::NOTHING;
                    for _ in 0..6 {
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    size,
                                )),
                                ..Default::default()
                            },
                            |ui| {
                                app.render_workspace(ui);
                                used = ui.min_rect();
                            },
                        );
                        output.textures_delta.clear();
                    }
                    assert!(
                        used.right() <= size.x + 1.0 && used.bottom() <= size.y + 1.0,
                        "{tab:?}, {theme:?}, {size:?}: content expanded outside viewport: {used:?}"
                    );
                }
            }
        }
        assert_eq!(app.filter.city, "Улан-Удэ");
        assert_eq!(app.filter.stack, "мебельные фабрики");
        assert_eq!(app.filter.max_items, 1_000);
        assert!(
            !app.default_json_path.exists(),
            "UI rendering must not create or save contacts"
        );
    }

    #[test]
    fn search_completion_reports_save_failure_and_keeps_results_in_memory() {
        let mut app = test_app();
        let (tx, rx) = channel();
        app.search_rx = Some(rx);
        app.is_searching = true;
        let mut record = app.contacts[0].clone();
        record.id = "isolated-new-contact".into();
        record.company = "Additional company".into();
        record.email = "new@example.org".into();
        record.url = "https://additional.example.org".into();
        record.phone.clear();
        tx.send(SearchProgress::Done(vec![record])).unwrap();
        app.poll_async_events();
        assert_eq!(app.contacts.len(), 16);
        assert!(!app.is_searching);
        assert!(app.status_message.contains("could not be saved"));
        assert!(app.logs.iter().any(|entry| entry.level == "ERROR"));
        assert!(!app.default_json_path.exists());
    }

    #[test]
    fn compact_workspace_shows_contact_rows_and_search_ignores_email_case() {
        let mut app = test_app();
        app.active_tab = ActiveTab::Contacts;
        app.contacts[0].email = "Contact0@Example.ORG".into();
        app.table_search_query = "contact0@example.org".into();
        for theme in [egui::ThemePreference::Dark] {
            let ctx = egui::Context::default();
            configure_ui(&ctx);
            ctx.set_theme(theme);
            let mut visible = Vec::new();
            for _ in 0..6 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(600.0, 400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| app.render_workspace(ui),
                );
                visible = output
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        if let egui::Shape::Text(text) = &shape.shape {
                            if shape.clip_rect.contains_rect(text.visual_bounding_rect()) {
                                return Some(text.galley.job.text.clone());
                            }
                        }
                        None
                    })
                    .collect::<Vec<_>>();
                output.textures_delta.clear();
            }
            assert!(
                visible.iter().any(|text| text == "Company / role"),
                "Table header is obscured: {visible:?}"
            );
            assert!(
                visible.iter().any(|text| text == "Contact0@Example.ORG"),
                "Matching row is missing or obscured: {visible:?}"
            );
            assert!(!visible.iter().any(|text| text == "contact1@example.org"));
        }
        assert!(!app.default_json_path.exists());
    }

    #[test]
    fn keyboard_navigation_and_delete_cancel_preserve_contacts() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        configure_ui(&ctx);
        let modifiers = egui::Modifiers::COMMAND;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1180.0, 780.0),
                )),
                events: vec![egui::Event::Key {
                    key: egui::Key::Num2,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
            |ui| app.render_workspace(ui),
        );
        output.textures_delta.clear();
        assert_eq!(app.active_tab, ActiveTab::Contacts);
        app.confirm_clear = true;
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.render_workspace(ui));
            output.textures_delta.clear();
        }
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.render_workspace(ui),
        );
        output.textures_delta.clear();
        assert!(!app.confirm_clear);
        assert_eq!(app.contacts.len(), 15);
        assert!(!app.default_json_path.exists());
    }

    #[test]
    fn blocked_sender_cannot_start_test_bulk_or_followup() {
        let mut app = test_app();
        app.smtp.direct_mx = false;
        app.smtp.username = "authenticated@gmail.com".into();
        app.smtp.from_email = "sample-user@vk.com".into();
        app.smtp.reply_to = app.smtp.from_email.clone();
        app.test_email_target = "recipient@example.org".into();
        for action in [
            AppState::send_test_email,
            AppState::start_mailing,
            AppState::start_followup_mailing,
        ] {
            action(&mut app);
            assert!(!app.is_mailing);
            assert!(app.mailer_rx.is_none());
            assert!(app.status_message.contains("VK Mail"));
        }
        app.smtp.direct_mx = true;
        app.smtp.host.clear();
        app.smtp.username.clear();
        app.smtp.password.clear();
        assert!(app.mail_settings_ready());
        let identity = mailer::validate_delivery_identity(&app.smtp, true).unwrap();
        assert_eq!(identity.from, "sample-user@vk.com");
        assert_eq!(identity.reply_to, identity.from);
        assert_eq!(identity.helo, "desktop.outreach.local");
    }

    #[test]
    fn test_delivery_releases_busy_state_without_changing_contacts() {
        let mut app = test_app();
        let before = serde_json::to_string(&app.contacts).unwrap();
        let (tx, rx) = channel();
        app.mailer_rx = Some(rx);
        app.is_mailing = true;
        app.send_test_email();
        assert!(app.mailer_rx.is_some());
        tx.send(MailerProgress::TestDone(Ok(()))).unwrap();
        app.poll_async_events();
        assert!(!app.is_mailing);
        assert!(app.mailer_rx.is_none());
        assert!(app.status_message.contains("accepted by the mail server"));
        assert_eq!(serde_json::to_string(&app.contacts).unwrap(), before);
        let (tx, rx) = channel();
        app.mailer_rx = Some(rx);
        app.is_mailing = true;
        tx.send(MailerProgress::TestDone(Err("connection failed".into())))
            .unwrap();
        app.poll_async_events();
        assert!(!app.is_mailing);
        assert!(app.status_message.contains("connection failed"));
    }
}
