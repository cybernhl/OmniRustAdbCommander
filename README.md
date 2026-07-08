# OmniRustAdbCommander

[English](#english) | [繁體中文](#繁體中文) | [简体中文](#简体中文)

---

## English

A high-performance, dual-pane desktop file manager written in Rust, designed for seamless file operations between local file systems and Android devices via a pluggable ADB backend.

### Features
- **Total Commander-style Dual Pane**: Independent file browsers, path navigation, and drive switchers on both sides.
- **Pluggable ADB Backend**: Modular communication supporting both official AOSP ADB and pure Rust implementations (`radb`).
- **Async Task Queue**: High-stability background processing for batch file transfers (Copy, Move, Push, Pull) without freezing the UI.
- **Cross-Platform**: Native performance on Windows, macOS, and Linux with low memory footprint.
- **VFS Layer**: Unified abstraction for local and remote file systems.

### Tech Stack
- **Language**: Rust
- **UI Framework**: `egui` (via `eframe`)
- **Async Runtime**: `tokio`
- **ADB Protocol**: `radb`
- **Data Serialization**: `serde`

---

## 繁體中文

基於 Rust 開發的高效能雙面板桌面檔案管理器，專為本地檔案系統與 Android 裝置之間的無縫操作而設計。

### 核心特性
- **致敬經典的雙面板介面**：左右兩側擁有完全獨立的檔案瀏覽、路徑導航與磁碟/裝置切換器。
- **可抽換式 ADB 後端**：通訊模組化設計，支援標準 AOSP ADB 或純 Rust 實現的 `radb`。
- **非同步任務佇列**：強大的背景處理系統，支援批次傳輸（複製、移動、上傳、下載），確保介面流暢不卡死。
- **跨平台原生體驗**：完美支援 Windows、macOS 與 Linux，具備高運算效率與極低記憶體占用。
- **虛擬檔案系統 (VFS)**：將本地與 Android 遠端檔案系統抽象化為統一操作介面。

### 技術棧
- **程式語言**：Rust
- **UI 框架**：`egui` (透過 `eframe`)
- **非同步運行時**：`tokio`
- **ADB 協議**：`radb`
- **序列化**：`serde`

---

## 简体中文

基于 Rust 开发的高性能双面板桌面文件管理器，专为本地文件系统与 Android 设备之间的无缝操作而设计。

### 核心特性
- **经典双面板界面**：左右两侧拥有完全独立的浏览器、路径导航与磁盘/设备切换器。
- **可插拔 ADB 后端**：通讯模块化设计，支持标准 AOSP ADB 或纯 Rust 实现的 `radb`。
- **异步任务队列**：强大的后台处理系统，支持批量传输（复制、移动、上传、下载），确保界面流畅不卡死。
- **跨平台原生体验**：完美支持 Windows、macOS 与 Linux，具备高效能与极低内存占用。
- **虚拟文件系统 (VFS)**：将本地与 Android 远程文件系统抽象化为统一操作接口。

### 技术栈
- **编程语言**：Rust
- **UI 框架**：`egui` (通过 `eframe`)
- **异步运行时**：`tokio`
- **ADB 协议**：`radb`
- **序列化**：`serde`
