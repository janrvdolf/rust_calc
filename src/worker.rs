use crate::eval;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Handle to a persistent background thread that evaluates expressions.
pub struct EvalWorker {
    job_tx: Sender<String>,
    result_rx: Receiver<Result<f64, String>>,
}

impl EvalWorker {
    /// Start the worker thread. It blocks on incoming jobs until `job_tx` is dropped.
    pub fn spawn() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<String>();
        let (result_tx, result_rx) = mpsc::channel();

        thread::spawn(move || {
            while let Ok(expr) = job_rx.recv() {
                thread::sleep(Duration::from_secs(2));
                let _ = result_tx.send(eval::evaluate(&expr));
            }
        });

        Self { job_tx, result_rx }
    }

    /// Submit an expression for evaluation.
    pub fn submit(&self, expr: String) {
        let _ = self.job_tx.send(expr);
    }

    /// Non-blocking poll for a completed result.
    pub fn try_recv(&self) -> Option<Result<f64, String>> {
        self.result_rx.try_recv().ok()
    }

    /// Discard any results already waiting in the channel.
    pub fn drain_results(&self) {
        while self.result_rx.try_recv().is_ok() {}
    }
}
