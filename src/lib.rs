pub mod grpc {
    tonic::include_proto!("notify.fsevent");
}

pub type FsNoticeWrite = grpc::NoticeWrite;
pub type FsNoticeRemove = grpc::NoticeRemove;
pub type FsCreate = grpc::Create;
pub type FsWrite = grpc::Write;
pub type FsChmod = grpc::Chmod;
pub type FsRemove = grpc::Remove;
pub type FsRename = grpc::Rename;
pub type FsRescan = grpc::Rescan;
pub type FsError = grpc::Error;

pub use anyhow::Result;
pub use grpc::fs_debounced_event::Event;
pub use grpc::fs_notify_client::FsNotifyClient;
pub use grpc::fs_notify_server::FsNotify;
pub use grpc::fs_notify_server::FsNotifyServer;
pub use grpc::FsDebouncedEvent;
pub use grpc::FsRequest;
use notify::DebouncedEvent;
pub use notify::DebouncedEvent::*;

impl From<DebouncedEvent> for FsDebouncedEvent {
    fn from(fs_event: DebouncedEvent) -> Self {
        match fs_event {
            NoticeWrite(path) => FsDebouncedEvent {
                event: Some(Event::NoticeWrite(FsNoticeWrite {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            NoticeRemove(path) => FsDebouncedEvent {
                event: Some(Event::NoticeRemove(FsNoticeRemove {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            Create(path) => FsDebouncedEvent {
                event: Some(Event::Create(FsCreate {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            Write(path) => FsDebouncedEvent {
                event: Some(Event::Write(FsWrite {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            Chmod(path) => FsDebouncedEvent {
                event: Some(Event::Chmod(FsChmod {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            Remove(path) => FsDebouncedEvent {
                event: Some(Event::Remove(FsRemove {
                    path: path.to_string_lossy().to_string(),
                })),
            },
            Rename(original, new_name) => FsDebouncedEvent {
                event: Some(Event::Rename(FsRename {
                    original: original.to_string_lossy().to_string(),
                    new_name: new_name.to_string_lossy().to_string(),
                })),
            },
            Rescan => FsDebouncedEvent {
                event: Some(Event::Rescan(FsRescan {})),
            },
            Error(msg, path) => FsDebouncedEvent {
                event: Some(Event::Error(FsError {
                    msg: msg.to_string(),
                    path: Some(
                        path.unwrap_or("Unknown Path!".into())
                            .to_string_lossy()
                            .to_string(),
                    ),
                })),
            },
        }
    }
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            tracing::error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            tracing::error!(message = %panic);
        }
    }));
}
