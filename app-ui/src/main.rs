use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use adb_explorer_backend::adb::AdbBackend;
use adb_explorer_backend::local::LocalBackend;
use adb_explorer_core::controller::ExplorerController;
use adb_explorer_common::models::FileEntry;
use adb_explorer_queue::manager::QueueManager;
use adb_explorer_queue::task::{TaskType, TaskStatus};
use adb_explorer_queue::events::TransferEvent;
use std::sync::mpsc::{Receiver, Sender, channel};
use radb::AdbClient;

enum AppMessage {
    DevicesLoaded(Vec<String>),
    PaneLoaded { pane_id: PaneSide, entries: Vec<FileEntry> },
    Error(String),
    TransferStarted(u64, String),
    TransferStatusUpdate(u64, TaskStatus),
    OperationFinished(PaneSide),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PaneSide {
    Left,
    Right,
}

#[derive(Clone, PartialEq, Eq)]
enum PaneType {
    Local,
    Adb(String),
}

struct PaneState {
    _side: PaneSide,
    pane_type: PaneType,
    controller: Arc<Mutex<ExplorerController>>,
    entries: Vec<FileEntry>,
    loading: bool,
    selected_index: Option<usize>,
}

#[derive(Clone)]
struct TransferDialog {
    from_side: PaneSide,
    entry: FileEntry,
    target_path: String,
    is_move: bool,
}

struct AdbExplorerApp {
    devices: Vec<String>,
    local_drives: Vec<(String, String)>,
    left_pane: PaneState,
    right_pane: PaneState,

    queue_manager: Option<Arc<Mutex<QueueManager>>>,

    tx: Sender<AppMessage>,
    rx: Receiver<AppMessage>,
    active_tasks: std::collections::HashMap<u64, (String, TaskStatus)>,

    // UI state for dialogs
    transfer_dialog: Option<TransferDialog>,
    new_folder_dialog: Option<(PaneSide, String)>,
    delete_dialog: Option<(PaneSide, FileEntry)>,
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Windows 字体路径
    #[cfg(windows)]
    {
        let font_paths = [
            "C:\\Windows\\Fonts\\msjh.ttc",
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ];
        for path in font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert("cjk_font".to_owned(), egui::FontData::from_owned(font_data));
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "cjk_font".to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("cjk_font".to_owned());
                break;
            }
        }
    }

    // macOS 字体路径
    #[cfg(target_os = "macos")]
    {
        let font_paths = [
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/PingFang.ttc",
        ];
        for path in font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert("cjk_font".to_owned(), egui::FontData::from_owned(font_data));
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "cjk_font".to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("cjk_font".to_owned());
                break;
            }
        }
    }

    ctx.set_fonts(fonts);
}

impl AdbExplorerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        let (tx, rx) = channel();

        let left_backend = Arc::new(LocalBackend::new());
        let mut left_controller = ExplorerController::new(left_backend);
        let left_start = if cfg!(windows) { "C:\\".to_string() } else { "/".to_string() };
        left_controller.set_path(left_start);

        let right_backend = Arc::new(LocalBackend::new());
        let mut right_controller = ExplorerController::new(right_backend);
        let right_start = if cfg!(windows) { "C:\\".to_string() } else { "/".to_string() };
        right_controller.set_path(right_start);

        let app = Self {
            devices: Vec::new(),
            local_drives: LocalBackend::get_logical_drives(),
            left_pane: PaneState {
                _side: PaneSide::Left,
                pane_type: PaneType::Local,
                controller: Arc::new(Mutex::new(left_controller)),
                entries: Vec::new(),
                loading: true,
                selected_index: None,
            },
            right_pane: PaneState {
                _side: PaneSide::Right,
                pane_type: PaneType::Local,
                controller: Arc::new(Mutex::new(right_controller)),
                entries: Vec::new(),
                loading: true,
                selected_index: None,
            },
            queue_manager: None,
            tx,
            rx,
            active_tasks: std::collections::HashMap::new(),
            transfer_dialog: None,
            new_folder_dialog: None,
            delete_dialog: None,
        };

        app.load_pane_directory(PaneSide::Left, false);
        app.load_pane_directory(PaneSide::Right, false);
        app.refresh_devices();
        app
    }

    fn setup_queue_listener(&self, qm: &mut QueueManager) {
        if let Some(mut rx) = qm.take_event_rx() {
            let tx = self.tx.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        TransferEvent::Started { task_id } => {
                            let _ = tx.send(AppMessage::TransferStatusUpdate(task_id, TaskStatus::Preparing));
                        }
                        TransferEvent::Progress { task_id, progress, speed } => {
                            let _ = tx.send(AppMessage::TransferStatusUpdate(task_id, TaskStatus::Running { progress, speed }));
                        }
                        TransferEvent::Finished { task_id } => {
                            let _ = tx.send(AppMessage::TransferStatusUpdate(task_id, TaskStatus::Finished));
                        }
                        TransferEvent::Error { task_id, message } => {
                            let _ = tx.send(AppMessage::TransferStatusUpdate(task_id, TaskStatus::Failed(message)));
                        }
                    }
                }
            });
        }
    }

    fn set_pane_adb(&mut self, side: PaneSide, serial: String, ctx: &egui::Context) {
        let backend = Arc::new(AdbBackend::new(serial.clone(), "127.0.0.1:5037".to_string()));

        if self.queue_manager.is_none() {
            let mut qm = QueueManager::new(backend.clone());
            self.setup_queue_listener(&mut qm);
            self.queue_manager = Some(Arc::new(Mutex::new(qm)));
        }

        let pane = match side {
            PaneSide::Left => &mut self.left_pane,
            PaneSide::Right => &mut self.right_pane,
        };

        pane.pane_type = PaneType::Adb(serial);
        pane.controller = Arc::new(Mutex::new(ExplorerController::new(backend)));
        self.navigate_to(side, "/sdcard".to_string(), ctx);
    }

    fn set_pane_local(&mut self, side: PaneSide, drive: Option<String>, ctx: &egui::Context) {
        let backend = Arc::new(LocalBackend::new());
        let start_path = drive.unwrap_or_else(|| if cfg!(windows) { "C:\\".to_string() } else { "/".to_string() });

        let pane = match side {
            PaneSide::Left => &mut self.left_pane,
            PaneSide::Right => &mut self.right_pane,
        };

        pane.pane_type = PaneType::Local;
        pane.controller = Arc::new(Mutex::new(ExplorerController::new(backend)));
        self.navigate_to(side, start_path, ctx);
    }

    fn refresh_devices(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match AdbClient::connect("127.0.0.1:5037").await {
                Ok(mut client) => {
                    match client.list_devices().await {
                        Ok(devices) => {
                            let serials = devices.into_iter().filter_map(|d| d.serial).collect();
                            let _ = tx.send(AppMessage::DevicesLoaded(serials));
                        }
                        Err(e) => { let _ = tx.send(AppMessage::Error(format!("Failed to list devices: {}", e))); }
                    }
                }
                Err(e) => { let _ = tx.send(AppMessage::Error(format!("ADB Server not found: {}", e))); }
            }
        });
    }

    fn load_pane_directory(&self, side: PaneSide, force_refresh: bool) {
        let pane = match side {
            PaneSide::Left => &self.left_pane,
            PaneSide::Right => &self.right_pane,
        };
        let controller = pane.controller.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut ctrl = controller.lock().await;
            match ctrl.list_current_dir(force_refresh).await {
                Ok(entries) => {
                    let _ = tx.send(AppMessage::PaneLoaded { pane_id: side, entries });
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Error(format!("Pane {:?} error: {}", side, e)));
                }
            }
        });
    }

    fn navigate_to(&mut self, side: PaneSide, path: String, ctx: &egui::Context) {
        let pane = match side {
            PaneSide::Left => &mut self.left_pane,
            PaneSide::Right => &mut self.right_pane,
        };
        pane.loading = true;
        let controller = pane.controller.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let mut ctrl = controller.lock().await;
            ctrl.set_path(path);
            match ctrl.list_current_dir(false).await {
                Ok(entries) => { let _ = tx.send(AppMessage::PaneLoaded { pane_id: side, entries }); }
                Err(e) => { let _ = tx.send(AppMessage::Error(e.to_string())); }
            }
            ctx.request_repaint();
        });
    }

    fn go_up(&mut self, side: PaneSide, ctx: &egui::Context) {
        let (path, ptype) = {
            let pane = match side {
                PaneSide::Left => &self.left_pane,
                PaneSide::Right => &self.right_pane,
            };
            (get_current_path_sync(&pane.controller), pane.pane_type.clone())
        };
        let is_win = cfg!(windows) && matches!(ptype, PaneType::Local);
        let sep = if is_win { "\\" } else { "/" };
        let mut parts: Vec<&str> = path.split(sep).filter(|s| !s.is_empty()).collect();
        if !parts.is_empty() {
            parts.pop();
            let new_path = if parts.is_empty() {
                if is_win { path.split(':').next().unwrap_or("C").to_string() + ":\\" }
                else { "/".to_string() }
            } else {
                if is_win { parts.join("\\") }
                else { format!("/{}", parts.join("/")) }
            };
            self.navigate_to(side, new_path, ctx);
        }
    }

    fn trigger_copy_dialog(&mut self, is_move: bool) {
        if let Some(idx) = self.left_pane.selected_index {
            let entry = self.left_pane.entries[idx].clone();
            let target_path = get_current_path_sync(&self.right_pane.controller);
            self.transfer_dialog = Some(TransferDialog { from_side: PaneSide::Left, entry, target_path, is_move });
        } else if let Some(idx) = self.right_pane.selected_index {
            let entry = self.right_pane.entries[idx].clone();
            let target_path = get_current_path_sync(&self.left_pane.controller);
            self.transfer_dialog = Some(TransferDialog { from_side: PaneSide::Right, entry, target_path, is_move });
        }
    }

    fn trigger_new_folder_dialog(&mut self) {
        self.new_folder_dialog = Some((PaneSide::Left, "New Folder".to_string()));
    }

    fn trigger_delete_dialog(&mut self) {
        if let Some(idx) = self.left_pane.selected_index {
            self.delete_dialog = Some((PaneSide::Left, self.left_pane.entries[idx].clone()));
        } else if let Some(idx) = self.right_pane.selected_index {
            self.delete_dialog = Some((PaneSide::Right, self.right_pane.entries[idx].clone()));
        }
    }

    fn execute_transfer(&mut self, dialog: TransferDialog, now: bool) {
        let qm = match &self.queue_manager { Some(q) => q.clone(), None => return };
        let (from_pane, to_pane) = match dialog.from_side {
            PaneSide::Left => (&self.left_pane, &self.right_pane),
            PaneSide::Right => (&self.right_pane, &self.left_pane),
        };
        let task = match (&from_pane.pane_type, &to_pane.pane_type) {
            (PaneType::Adb(_), PaneType::Local) => {
                let sep = if cfg!(windows) { "\\" } else { "/" };
                let local_dest = format!("{}{}{}", dialog.target_path.trim_end_matches(sep), sep, dialog.entry.name);
                Some(TaskType::Pull { remote: dialog.entry.full_path, local: local_dest })
            }
            (PaneType::Local, PaneType::Adb(_)) => {
                let remote_dest = format!("{}/{}", dialog.target_path.trim_end_matches('/'), dialog.entry.name);
                Some(TaskType::Push { local: dialog.entry.full_path, remote: remote_dest })
            }
            _ => None,
        };
        if let Some(task) = task {
            let tx = self.tx.clone();
            let desc = format!("{:?}", task);
            tokio::spawn(async move {
                let mut qm_lock = qm.lock().await;
                let id = qm_lock.add_task(task, now).await;
                let _ = tx.send(AppMessage::TransferStarted(id, desc));
            });
        }
    }
}

fn get_current_path_sync(controller: &Arc<Mutex<ExplorerController>>) -> String {
    if let Ok(c) = controller.try_lock() {
        c.current_path().to_string()
    } else {
        "...".to_string()
    }
}

fn format_size(size: u64) -> String {
    if size == 0 { return "".to_string(); }
    if size < 1024 { format!("{} B", size) }
    else if size < 1024 * 1024 { format!("{:.1} K", size as f64 / 1024.0) }
    else { format!("{:.1} M", size as f64 / 1024.0 / 1024.0) }
}

fn format_time(timestamp: Option<u64>) -> String {
    timestamp.map(|t| {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(t as i64, 0).unwrap_or_default();
        dt.format("%Y-%m-%d %H:%M").to_string()
    }).unwrap_or_else(|| "---".to_string())
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len {
        text.to_string()
    } else {
        // 中間截斷，保留開頭與後半部
        let keep_start = (max_len / 2).max(1);
        let keep_end = (max_len - keep_start - 3).max(1);
        let start: String = text.chars().take(keep_start).collect();
        let end: String = text.chars().skip(char_count - keep_end).collect();
        format!("{}...{}", start, end)
    }
}

impl eframe::App for AdbExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::DevicesLoaded(serials) => { self.devices = serials; }
                AppMessage::PaneLoaded { pane_id, entries } => {
                    let pane = match pane_id {
                        PaneSide::Left => &mut self.left_pane,
                        PaneSide::Right => &mut self.right_pane,
                    };
                    pane.entries = entries;
                    pane.loading = false;
                    pane.selected_index = None;
                }
                AppMessage::Error(e) => { eprintln!("Error: {}", e); }
                AppMessage::TransferStarted(id, desc) => {
                    self.active_tasks.insert(id, (desc, TaskStatus::Queued));
                }
                AppMessage::TransferStatusUpdate(id, status) => {
                    if let Some(task) = self.active_tasks.get_mut(&id) {
                        task.1 = status.clone();
                        if matches!(status, TaskStatus::Finished) {
                            self.load_pane_directory(PaneSide::Left, true);
                            self.load_pane_directory(PaneSide::Right, true);
                        }
                    }
                }
                AppMessage::OperationFinished(side) => { self.load_pane_directory(side, true); }
            }
            ctx.request_repaint();
        }

        // --- BOTTOM F-KEYS ---
        egui::TopBottomPanel::bottom("f_keys").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let button_width = (ui.available_width() - 40.0) / 7.0;
                if ui.add_sized([button_width, 30.0], egui::Button::new("F3 View")).clicked() {}
                if ui.add_sized([button_width, 30.0], egui::Button::new("F4 Edit")).clicked() {}
                if ui.add_sized([button_width, 30.0], egui::Button::new("F5 Copy")).clicked() { self.trigger_copy_dialog(false); }
                if ui.add_sized([button_width, 30.0], egui::Button::new("F6 Move")).clicked() { self.trigger_copy_dialog(true); }
                if ui.add_sized([button_width, 30.0], egui::Button::new("F7 NewFolder")).clicked() { self.trigger_new_folder_dialog(); }
                if ui.add_sized([button_width, 30.0], egui::Button::new("F8 Delete")).clicked() { self.trigger_delete_dialog(); }
                if ui.add_sized([button_width, 30.0], egui::Button::new("Alt+F4 Exit")).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            });
        });

        // --- CENTRAL PANES ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                self.show_pane(&mut columns[0], PaneSide::Left, ctx);
                self.show_pane(&mut columns[1], PaneSide::Right, ctx);
            });
        });

        self.show_dialogs(ctx);
        if !self.active_tasks.is_empty() { self.show_floating_progress(ctx); }
    }
}

impl AdbExplorerApp {
    fn show_pane(&mut self, ui: &mut egui::Ui, side: PaneSide, ctx: &egui::Context) {
        let (current_path, current_type, loading, entries, selected_index) = {
            let pane = match side {
                PaneSide::Left => &self.left_pane,
                PaneSide::Right => &self.right_pane,
            };
            (get_current_path_sync(&pane.controller), pane.pane_type.clone(), pane.loading, pane.entries.clone(), pane.selected_index)
        };

        ui.vertical(|ui| {
            // DRIVE SELECTOR (Row 1)
            ui.horizontal(|ui| {
                let drive_letter = match &current_type {
                    PaneType::Local => {
                        current_path.chars().next().unwrap_or('C').to_ascii_uppercase().to_string()
                    },
                    PaneType::Adb(_) => "ADB".to_string(),
                };

                // 按下 spinner 顯示全部 (代號顯示在 spinner 上)
                ui.menu_button(format!("{} v", drive_letter), |ui| {
                    ui.label("Local Drives:");
                    for (root, label) in self.local_drives.clone() {
                        let display = if label.is_empty() {
                            format!("💻 [{}]", root.trim_end_matches('\\'))
                        } else {
                            format!("💻 [{}] {}", root.trim_end_matches('\\'), label)
                        };
                        if ui.button(display).clicked() {
                            self.set_pane_local(side, Some(root), ctx);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    ui.label("ADB Devices:");
                    for serial in self.devices.clone() {
                        if ui.button(format!("📱 {}", serial)).clicked() {
                            self.set_pane_adb(side, serial, ctx);
                            ui.close_menu();
                        }
                    }
                    if ui.button("🔄 Refresh List").clicked() { self.refresh_devices(); }
                });

                let volume_label = match &current_type {
                    PaneType::Local => {
                        let letter = drive_letter.chars().next().unwrap_or('C');
                        self.local_drives.iter()
                            .find(|(root, _)| root.starts_with(letter))
                            .map(|(_, l)| if l.is_empty() { String::new() } else { format!("[{}]", l) })
                            .unwrap_or_default()
                    },
                    PaneType::Adb(serial) => format!("[{}]", serial),
                };

                // 名稱顯示在打X的地方 (靠右側)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(volume_label).strong());
                });
            });

            // 快速磁碟按鈕 (Row 2) - 只有顯示目前選中的
            ui.horizontal(|ui| {
                if matches!(current_type, PaneType::Local) {
                    let active_drive = current_path.chars().next().unwrap_or('C').to_ascii_lowercase();
                    let text = format!("{}:", active_drive);
                    ui.add(egui::Button::new(egui::RichText::new(text).strong()).fill(egui::Color32::from_rgb(180, 180, 180)));
                }
            });

            // PATH BAR (Row 3)
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&current_path).strong().color(egui::Color32::DARK_BLUE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⮬").clicked() { self.go_up(side, ctx); }
                });
            });

            ui.separator();

            if loading {
                ui.centered_and_justified(|ui| { ui.spinner(); });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt(format!("scroll_{:?}", side))
                    .show(ui, |ui| {

                    ui.horizontal(|ui| {
                        ui.set_min_height(20.0);
                        ui.add_sized([ui.available_width() * 0.45, 20.0], egui::Label::new(egui::RichText::new("Name").strong()));
                        ui.add_sized([40.0, 20.0], egui::Label::new(egui::RichText::new("Ext").strong()));
                        ui.add_sized([60.0, 20.0], egui::Label::new(egui::RichText::new("Size").strong()));
                        ui.add_sized([110.0, 20.0], egui::Label::new(egui::RichText::new("Date").strong()));
                    });
                    ui.separator();

                    for (i, entry) in entries.iter().enumerate() {
                        let is_selected = selected_index == Some(i);
                        let icon = if entry.is_dir { "📁" } else { "📄" };

                        let (name_part, ext_part) = if entry.is_dir {
                            (entry.name.clone(), "".to_string())
                        } else {
                            match entry.name.rfind('.') {
                                Some(idx) => (entry.name[..idx].to_string(), entry.name[idx+1..].to_string()),
                                None => (entry.name.clone(), "".to_string()),
                            }
                        };

                        let formatted_size = if entry.is_dir { "<DIR>".to_string() } else { format_size(entry.size) };
                        let formatted_date = format_time(entry.modified);

                        let row_response = ui.horizontal(|ui| {
                            let total_w = ui.available_width();
                            let truncated_name = truncate_text(&name_part, 40);
                            let res = ui.add_sized([total_w * 0.45, 20.0], egui::SelectableLabel::new(is_selected, format!("{} {}", icon, truncated_name)));
                            ui.add_sized([40.0, 20.0], egui::Label::new(&ext_part));
                            ui.add_sized([60.0, 20.0], egui::Label::new(&formatted_size));
                            ui.add_sized([110.0, 20.0], egui::Label::new(&formatted_date));
                            res
                        }).inner;

                        if row_response.clicked() {
                            match side {
                                PaneSide::Left => self.left_pane.selected_index = Some(i),
                                PaneSide::Right => self.right_pane.selected_index = Some(i),
                            }
                        }
                        if row_response.double_clicked() && entry.is_dir {
                            self.navigate_to(side, entry.full_path.clone(), ctx);
                        }
                    }
                });
            }
        });
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(dialog) = self.transfer_dialog.clone() {
            egui::Window::new(if dialog.is_move { "Move" } else { "Copy" })
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("{} \"{}\" to:", if dialog.is_move { "Move" } else { "Copy" }, dialog.entry.name));
                    let mut path = dialog.target_path.clone();
                    ui.text_edit_singleline(&mut path);

                    ui.horizontal(|ui| {
                        if ui.button("OK (Now)").clicked() {
                            let mut d = dialog.clone(); d.target_path = path.clone();
                            self.execute_transfer(d, true);
                            self.transfer_dialog = None;
                        }
                        if ui.button("F2 Queue").clicked() {
                            let mut d = dialog.clone(); d.target_path = path;
                            self.execute_transfer(d, false);
                            self.transfer_dialog = None;
                        }
                        if ui.button("Cancel").clicked() { self.transfer_dialog = None; }
                    });
                });
        }

        if let Some((side, input)) = self.new_folder_dialog.clone() {
            let mut current_input = input;
            egui::Window::new("New Folder").show(ctx, |ui| {
                ui.text_edit_singleline(&mut current_input);
                if ui.button("Create").clicked() {
                    let _ = side; // Implement mkdir
                    self.new_folder_dialog = None;
                }
                if ui.button("Cancel").clicked() { self.new_folder_dialog = None; }
            });
            if self.new_folder_dialog.is_some() {
                self.new_folder_dialog = Some((side, current_input));
            }
        }

        if let Some((_side, entry)) = self.delete_dialog.clone() {
            egui::Window::new("Delete").show(ctx, |ui| {
                ui.label(format!("Are you sure you want to delete {}?", entry.name));
                if ui.button("Delete").clicked() {
                    self.delete_dialog = None;
                }
                if ui.button("Cancel").clicked() { self.delete_dialog = None; }
            });
        }
    }

    fn show_floating_progress(&mut self, ctx: &egui::Context) {
        egui::Window::new("Background Tasks")
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -100.0])
            .collapsible(true)
            .show(ctx, |ui| {
                let mut tasks: Vec<_> = self.active_tasks.iter().collect();
                tasks.sort_by_key(|(&id, _)| id);
                for (id, (desc, status)) in tasks {
                    ui.group(|ui| {
                        ui.label(format!("[{}] {}", id, desc));
                        match status {
                            TaskStatus::Running { progress, .. } => {
                                ui.add(egui::ProgressBar::new(*progress).show_percentage());
                            }
                            TaskStatus::Finished => { ui.colored_label(egui::Color32::GREEN, "Finished"); }
                            TaskStatus::Failed(e) => { ui.colored_label(egui::Color32::RED, e); }
                            _ => { ui.label(format!("{:?}", status)); }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("▶ Start Queue").clicked() {
                        if let Some(qm) = &self.queue_manager {
                            let qm = qm.clone();
                            tokio::spawn(async move { qm.lock().await.start_queue().await; });
                        }
                    }
                    if ui.button("Clear Finished").clicked() {
                        self.active_tasks.retain(|_, (_, s)| !matches!(s, TaskStatus::Finished));
                    }
                });
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 900.0])
            .with_title("OmniRustAdbCommander"),
        ..Default::default()
    };
    eframe::run_native("OmniRustAdbCommander", options, Box::new(|cc| { Ok(Box::new(AdbExplorerApp::new(cc))) }))
}
