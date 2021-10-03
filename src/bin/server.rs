use monitor::*;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use parking_lot::{Condvar, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing_subscriber::{self, fmt, subscribe::CollectExt, EnvFilter};

static TASK_QUENE: Lazy<
    Arc<
        Mutex<
            Vec<(
                Receiver<DebouncedEvent>,
                UnboundedSender<Result<FsDebouncedEvent, Status>>,
            )>,
        >,
    >,
> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

static PAIR: Lazy<Arc<(Mutex<bool>, Condvar)>> =
    Lazy::new(|| Arc::new((Mutex::new(false), Condvar::new())));

struct Delegate;

#[tonic::async_trait]
impl FsNotify for Delegate {
    type ListenStream = UnboundedReceiverStream<Result<FsDebouncedEvent, Status>>;
    async fn listen(
        &self,
        req: Request<FsRequest>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        let (tx, rx) = mpsc::unbounded_channel();

        TASK_QUENE.lock().push((
            thread::spawn(move || {
                let (fs_tx, fs_rx) = channel();
                let mut fs_watcher = watcher(fs_tx, Duration::from_millis(500)).unwrap();
                fs_watcher
                    .watch(req.into_inner().dir_path, RecursiveMode::Recursive)
                    .unwrap();
                std::mem::forget(fs_watcher);
                fs_rx
            })
            .join()
            .unwrap(),
            tx,
        ));

        let pair2 = Arc::clone(&PAIR);
        let &(ref lock, ref cvar) = &*pair2;
        let mut active = lock.lock();
        *active = true;
        cvar.notify_one();
        *active = false;

        Ok(Response::new(UnboundedReceiverStream::new(rx)))
    }
}

fn init_wait_queue() {
    let pair1 = Arc::clone(&PAIR);

    thread::spawn(move || loop {
        let &(ref lock, ref cvar) = &*pair1;
        let mut active = lock.lock();
        cvar.wait(&mut active);
        while let Some((fs_rx, tx)) = TASK_QUENE.lock().pop() {
            thread::spawn(move || loop {
                match fs_rx.recv() {
                    Ok(event) => {
                        tx.send(Ok(FsDebouncedEvent::from(event))).unwrap();
                    }
                    Err(msg) => {
                        tx.send(Ok(FsDebouncedEvent {
                            event: Some(Event::Error(FsError {
                                msg: msg.to_string(),
                                path: None,
                            })),
                        }))
                        .unwrap();
                    }
                }
            });
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    set_panic_hook();
    let (non_blocking, _guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily("./", "Crash.log"));

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
    init_wait_queue();
    let service = FsNotifyServer::new(Delegate);
    Server::builder()
        .add_service(service)
        .serve(dotenv::var("LISTEN_ADDR")?.parse()?)
        .await?;

    Ok(())
}
