#![feature(type_alias_impl_trait)]
use once_cell::sync::Lazy;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::ReusableBoxFuture;

#[derive(Debug)]
pub enum ControlMessage {
    Stop,
}

#[derive(Debug)]
pub struct BackgroundTask {
    pub name: String,
    pub task: ReusableBoxFuture<'static, ()>,
}

#[derive(Debug, Clone)]
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

static GLOBAL_EVENT_LOOP: Lazy<(
    tokio::sync::mpsc::Sender<BackgroundTask>,
    tokio::sync::mpsc::Sender<ControlMessage>,
)> = Lazy::new(|| {
    let (queue, queue_r) = tokio::sync::mpsc::channel(100);
    let (control, control_r) = tokio::sync::mpsc::channel(100);
    futures::executor::block_on(async move {
        tokio::spawn(BackgroundWorker::event_loop(queue_r, control_r));
    });
    (queue, control)
});

pub async fn batch_execute_background_tasks(tasks: Vec<BackgroundTask>) {
    let tasks = tasks
        .into_iter()
        .map(|task| GLOBAL_EVENT_LOOP.0.send(task))
        .collect::<Vec<_>>();
    futures::future::join_all(tasks).await;
}

pub async fn execute_background_task(task: BackgroundTask) {
    GLOBAL_EVENT_LOOP.0.send(task).await.unwrap();
}
