use monitor::{
    set_panic_hook, Event as FsEvent, FsChmod, FsCreate, FsError, FsNoticeRemove, FsNoticeWrite,
    FsNotifyClient, FsRemove, FsRename, FsRequest, FsWrite, Result,
};
use tonic::transport::Channel;
use tracing::*;
use tracing_subscriber::{self, fmt, subscribe::CollectExt, EnvFilter};

async fn get_remote_fsevents(client: &mut FsNotifyClient<Channel>, watch_dir: &str) -> Result<()> {
    let mut stream = client
        .listen(FsRequest {
            dir_path: watch_dir.into(),
        })
        .await?
        .into_inner();

    println!("connected");

    while let Some(msg) = stream.message().await? {
        if let Some(event) = msg.event {
            match event {
                FsEvent::NoticeWrite(e) => {
                    let FsNoticeWrite { path } = e;
                    trace!("[NoticeWrite]: {}", path);
                }
                FsEvent::NoticeRemove(e) => {
                    let FsNoticeRemove { path } = e;
                    trace!("[NoticeRemove]: {}", path);
                }
                FsEvent::Create(e) => {
                    let FsCreate { path } = e;
                    trace!("[Create]: {}", path);
                }
                FsEvent::Write(e) => {
                    let FsWrite { path } = e;
                    trace!("[Write]: {}", path);
                }
                FsEvent::Chmod(e) => {
                    let FsChmod { path } = e;
                    trace!("[Chmod]: {}", path);
                }
                FsEvent::Remove(e) => {
                    let FsRemove { path } = e;
                    trace!("[Remove]: {}", path);
                }
                FsEvent::Rename(e) => {
                    let FsRename { original, new_name } = e;
                    trace!("[Rename]: {} -> {}", original, new_name);
                }
                FsEvent::Rescan(_) => {
                    trace!("[Rescan]");
                }
                FsEvent::Error(e) => {
                    let FsError { msg, path } = e;
                    error!("[ERROR]: {} - {}", msg, path.unwrap_or("Unknown".into()));
                }
            }
        }
    }

    Ok(())
}

async fn run(watch_dir: String) -> Result<()> {
    let channel = Channel::builder(dotenv::var("REMOTE_ADDR")?.parse()?)
        .connect()
        .await?;
    let mut client = FsNotifyClient::new(channel).send_gzip().accept_gzip();

    get_remote_fsevents(&mut client, &watch_dir).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    set_panic_hook();

    let (non_blocking, _guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(
        dotenv::var("LOG_DIR")?,
        "FsChange.log",
    ));

    tracing::collect::set_global_default(
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
            .with(
                fmt::Subscriber::new()
                    .with_ansi(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_writer(non_blocking),
            ),
    )?;

    futures::future::join_all(
        dotenv::var("WATCH_DIRS")?
            .split(',')
            .map(str::trim)
            .map(str::to_string)
            .map(run)
            .map(tokio::spawn),
    )
    .await;
    Ok(())
}
