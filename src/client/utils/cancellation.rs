use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// A guard that executes an asynchronous callback and ensures a cancellation
/// callback is executed if the guard is dropped before the operation completes.
pub struct CancellationGuard {
    notify: Option<Arc<Notify>>,
    cancel: JoinHandle<()>,
}

impl CancellationGuard {
    fn new<C>(on_cancellation: C) -> Self
    where
        C: FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
    {
        let notify = Arc::new(Notify::new());

        let cancel = {
            let notify = Arc::clone(&notify);

            tokio::spawn(async move {
                notify.notified().await;
                on_cancellation().await;
            })
        };

        Self {
            notify: Some(notify),
            cancel,
        }
    }

    fn finish(&mut self) {
        self.notify.take();
        self.cancel.abort();
    }

    /// Wrap a callable with a cancellation guard, and automatically trigger
    /// a cancel message to a language server if the guard goes out of scope.
    pub async fn with_guard<F, C, O>(func: F, on_cancellation: C) -> O
    where
        F: AsyncFnOnce() -> O,
        C: FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
    {
        let mut guard = Self::new(on_cancellation);

        let response = func().await;

        guard.finish();

        response
    }
}

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        if let Some(notify) = self.notify.take() {
            notify.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn test_guard_runs_cancellation_on_drop() {
        let (started_tx, started_rx) = oneshot::channel();
        let (cancelled_tx, cancelled_rx) = oneshot::channel();

        let task = tokio::spawn(CancellationGuard::with_guard(
            async move || {
                // Signal that the task has started, then wait indefinitely
                started_tx
                    .send(())
                    .expect("Should be able to send started signal");

                std::future::pending::<()>().await;
            },
            move || {
                // Signal that the cancellation callback has been called
                Box::pin(async move {
                    cancelled_tx
                        .send(())
                        .expect("Should be able to send cancellation signal");
                })
            },
        ));

        started_rx.await.unwrap();

        task.abort();

        assert!(
            tokio::time::timeout(Duration::from_millis(100), cancelled_rx)
                .await
                .is_ok(),
            "Cancellation callback should have been called within 100ms of task abortion"
        );
    }

    #[tokio::test]
    async fn test_guard_doesnt_cancel_successful_task() {
        let (started_tx, mut started_rx) = oneshot::channel();
        let (cancelled, mut cancelled_rx) = oneshot::channel();

        CancellationGuard::with_guard(
            async move || {
                started_tx
                    .send(())
                    .expect("Should be able to send started signal");

                tokio::time::sleep(Duration::from_millis(100)).await;
            },
            move || {
                Box::pin(async move {
                    cancelled.send(()).unwrap();
                })
            },
        )
        .await;

        assert_eq!(
            started_rx.try_recv(),
            Ok(()),
            "Started callback should have been called",
        );
        assert_eq!(
            cancelled_rx.try_recv(),
            Err(oneshot::error::TryRecvError::Empty),
            "Cancellation callback should not have been called",
        );
    }
}
