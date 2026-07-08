
# Android ADB Explorer Implementation Plan

## Goal
Build a Total Commander-like Android file explorer in Rust.

## Architecture
UI
-> ExplorerController
-> QueueManager
-> Scheduler
-> Worker
-> DeviceBackend trait
-> RadbBackend / AOSPBackend

                     UI (egui / Slint / Tauri...)
                             │
                             │
                     Explorer Controller
                             │
              ┌──────────────┴──────────────┐
              │                             │
         Queue Manager                 File Browser
              │                             │
              │                     Virtual FileSystem
              │                             │
        Task Executor                Path Navigator
              │                             │
              └──────────────┬──────────────┘
                             │
                      Device Backend Trait
                             │
       ┌─────────────────────┴───────────────────┐
       │                                         │
      radb backend                     official adb backend
       │                                         │
libadb (submodule)                     packages/modules/adb

├── app-ui/                  # egui / Slint UI
├── explorer-core/
│   ├── controller/
│   ├── selection/
│   ├── navigation/
│   ├── filters/
│   ├── cache/
│   └── models/
├── queue/
│   ├── manager/
│   ├── scheduler/
│   ├── worker/
│   ├── task/
│   └── events/
├── backend/
│   ├── traits/
│   ├── adb-radb/
│   ├── adb-aosp/
│   └── mock/
├── media-scanner/
├── common/
└── third_party/
├── radb/                 # Git submodule
└── adb-aosp/             # Git submodule

## DeviceBackend
```rust
trait DeviceBackend {
 list_dir();
 stat();
 push();
 pull();
 mkdir();
 delete();
 rename();
}
```

## FileEntry
```rust
struct FileEntry {
 name:String,
 full_path:String,
 size:u64,
 is_hidden:bool,
}
```

Selection should be maintained by SelectionModel(HashSet<PathBuf>) instead of bool.

## Queue

States:
- Queued
- Preparing
- Running
- Verify
- MediaScan
- Finished
- Failed
- Cancelled
- Paused

Pseudocode:

```text
UI -> QueueManager.add(task)

Scheduler:
 while true:
   task = next()
   Worker.execute(task)

Worker:
 Preparing
 if push:
   mkdir_if_needed()
   upload(progress)
   verify(optional)
   delete_source(optional)
   media_scan(optional)
 emit Finished
```

## Events

```text
Started
Progress
Speed
ETA
Finished
Error
```

UI subscribes only to events.

## Filters
Use FilterRule trait instead of hardcoded HashSet.

## Cache
DirectoryCache stores directory listing.
Refresh only on explicit reload.

## Milestones
P1 Trait + MockBackend
P2 RadbBackend list/stat
P3 Browser+Selection+Cache
P4 Queue
P5 Push/Pull
P6 Advanced ops
P7 Plugin adapter
