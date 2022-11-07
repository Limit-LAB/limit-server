#![feature(type_alias_impl_trait)]

use motore::Service;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::ReusableBoxFuture;
use volo_grpc::Request;

#[derive(Debug)]
pub enum ControlMessage {
    Stop,
}

#[derive(Debug)]
pub struct BackgroundTask {
    pub name: String,
    pub task: ReusableBoxFuture<'static, ()>,
}

struct BackgroundWorker;

impl BackgroundWorker {
    async fn event_loop(
        mut queue: Receiver<BackgroundTask>,
        mut control: Receiver<ControlMessage>,
    ) {
        loop {
            tokio::select! {
                Some(task) = queue.recv() => {
                    tracing::info!("🚀 background task {} started", task.name);
                    tokio::spawn(task.task);
                }
                Some(msg) = control.recv() => {
                    tracing::info!("received control message: {:?}", msg);
                    match msg {
                        ControlMessage::Stop => break,
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(300)) => {
                }
            }
        }
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_EVENT_LOOP: (tokio::sync::mpsc::Sender<BackgroundTask>, tokio::sync::mpsc::Sender<ControlMessage>) = {
        let (queue, queue_r) = tokio::sync::mpsc::channel(100);
        let (control, control_r) = tokio::sync::mpsc::channel(100);
        tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            tokio::spawn(BackgroundWorker::event_loop(queue_r, control_r));
        });
        (queue, control)
    };
}

pub struct BackgroundTaskService<T> {
    inner: T,
}

#[motore::service]
impl<Cx, Req, I> Service<Cx, Request<Req>> for BackgroundTaskService<I>
where
    Req: Send + 'static,
    I: Service<Cx, Request<Req>> + Send + 'static,
    Cx: Send + 'static,
{
    async fn call(&mut self, cx: &mut Cx, mut req: Request<Req>) -> Result<I::Response, I::Error> {
        let tasks: Vec<BackgroundTask> = req.extensions_mut().remove().unwrap();
        let tesks = tasks
            .into_iter()
            .map(|task| GLOBAL_EVENT_LOOP.0.send(task))
            .collect::<Vec<_>>();
        futures::future::join_all(tesks).await;
        self.inner.call(cx, req).await
    }
}

pub struct BackgroundTaskLayer;

impl<S> volo::Layer<S> for BackgroundTaskLayer {
    type Service = BackgroundTaskService<S>;

    fn layer(self, inner: S) -> Self::Service {
        BackgroundTaskService { inner }
    }
}
