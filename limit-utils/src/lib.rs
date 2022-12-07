use std::{error::Error, fmt::Debug, future::Future};

use limit_deps::{futures::FutureExt, metrics::histogram, tokio::time::Instant, *};

use once_cell::sync::Lazy;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::ReusableBoxFuture;

#[derive(Debug)]
pub enum ControlMessage {
    Stop,
}

#[derive(Debug)]
pub struct BackgroundTask {
    name: String,
    task: ReusableBoxFuture<'static, ()>,
}

impl BackgroundTask {
    pub fn new<I, T, E, F>(name: I, task: F) -> Self
    where
        I: Into<String>,
        T: Debug,
        E: Error,
        F: Future<Output = Result<T, E>> + Send + 'static,
    {
        let name = name.into();
        let name2 = name.clone();
        Self {
            task: ReusableBoxFuture::new(async {
                let m = Measurement::start(&name2);
                task.map(move |r| match r {
                    Ok(t) => {
                        tracing::info!("{name2} success, returned ({t:?})");
                    }
                    Err(e) => {
                        tracing::error!("{name2} failed: {e}");
                    }
                })
                .await;
                m.end();
            }),
            name,
        }
    }
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
                    tracing::info!("background task {} started", task.name);
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

/// A guard that record multiple histograms on demand or on dropped. To properly
/// record measurements without any noise, remember use [`Measurement::end`], or
/// an `early_exit` event will be recorded.
pub struct Measurement {
    name: String,
    start: Instant,
    ongoing: bool,
}

impl Measurement {
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            ongoing: true,
        }
    }

    /// Record current elapsed time and start a new measurement **without reset** the timer
    pub fn record(&mut self, new_name: impl Into<String>) -> &mut Self {
        histogram!(self.name.clone(), self.start.elapsed());
        self.name = new_name.into();
        self
    }

    /// Record current elapsed time, start a new measurement and **reset** the timer
    pub fn renew(&mut self, new_name: impl Into<String>) -> &mut Self {
        histogram!(self.name.clone(), self.start.elapsed());
        self.name = new_name.into();
        self.start = Instant::now();
        self
    }

    /// Record current elapsed time and stop the measurement
    pub fn end(mut self) {
        let s = std::mem::take(&mut self.name);
        histogram!(s, self.start.elapsed());
        self.ongoing = false;
    }
}

impl Drop for Measurement {
    fn drop(&mut self) {
        if self.ongoing {
            let s = std::mem::take(&mut self.name);
            histogram!(s, self.start.elapsed(), "status" => "early_exit");
        }
    }
}
