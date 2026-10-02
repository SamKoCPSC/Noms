//! Cancellation and deadline ownership for browser image operations.

use std::{
    cell::RefCell,
    future::Future,
    rc::{Rc, Weak},
    task::Poll,
};

/// Own the result channel without letting JavaScript callbacks retain it.
/// A cancelled wait drops the owner; late completions simply drop their value.
pub(super) struct ImageCompletionOwner<T>(Rc<RefCell<Option<futures_channel::oneshot::Sender<T>>>>);
pub(super) struct ImageCompletion<T>(Weak<RefCell<Option<futures_channel::oneshot::Sender<T>>>>);

impl<T> ImageCompletionOwner<T> {
    pub(super) fn channel() -> (Self, futures_channel::oneshot::Receiver<T>) {
        let (sender, receiver) = futures_channel::oneshot::channel();
        (Self(Rc::new(RefCell::new(Some(sender)))), receiver)
    }

    pub(super) fn callback(&self) -> ImageCompletion<T> {
        ImageCompletion(Rc::downgrade(&self.0))
    }
}

impl<T> ImageCompletion<T> {
    pub(super) fn complete(self, value: T) {
        if let Some(owner) = self.0.upgrade() {
            if let Some(sender) = owner.borrow_mut().take() {
                let _ = sender.send(value);
            }
        }
    }
}

/// Race an operation against its deadline. Dropping this future drops both;
/// completion wins if both become ready together, avoiding a false timeout.
pub(super) async fn wait_for_image_operation<Operation, Deadline>(
    operation: Operation,
    deadline: Deadline,
    timeout_message: &str,
) -> Result<Operation::Output, String>
where
    Operation: Future,
    Deadline: Future<Output = ()>,
{
    let mut operation = std::pin::pin!(operation);
    let mut deadline = std::pin::pin!(deadline);
    std::future::poll_fn(|context| {
        if let Poll::Ready(result) = operation.as_mut().poll(context) {
            return Poll::Ready(Ok(result));
        }
        if deadline.as_mut().poll(context).is_ready() {
            return Poll::Ready(Err(timeout_message.to_string()));
        }
        Poll::Pending
    })
    .await
}

/// The decoder owns its temporary URL, including while suspended at an await.
/// Unlike cleanup after the await, Drop also runs when the caller is cancelled.
pub(super) struct ImageDecodeUrl {
    url: String,
    revoke: fn(&str),
}

impl ImageDecodeUrl {
    pub(super) fn new(url: String, revoke: fn(&str)) -> Self {
        Self { url, revoke }
    }

    pub(super) fn as_str(&self) -> &str {
        &self.url
    }
}

impl Drop for ImageDecodeUrl {
    fn drop(&mut self) {
        (self.revoke)(&self.url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        future::Future,
        pin::Pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    struct PendingOperation(Rc<Cell<usize>>);
    impl Future for PendingOperation {
        type Output = ();
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            Poll::Pending
        }
    }
    impl Drop for PendingOperation {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn completed_image_operation_wins_and_drops_its_deadline() {
        let drops = Rc::new(Cell::new(0));
        let mut operation = Box::pin(wait_for_image_operation(
            std::future::ready(42),
            PendingOperation(drops.clone()),
            "Timed out",
        ));
        assert_eq!(
            operation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(42))
        );
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn image_deadline_returns_error_and_drops_pending_operation() {
        let drops = Rc::new(Cell::new(0));
        let mut operation = Box::pin(wait_for_image_operation(
            PendingOperation(drops.clone()),
            std::future::ready(()),
            "Image encoding timed out",
        ));
        assert_eq!(
            operation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err("Image encoding timed out".into()))
        );
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn cancelling_image_wait_drops_operation_and_deadline() {
        let drops = Rc::new(Cell::new(0));
        let mut operation = Box::pin(wait_for_image_operation(
            PendingOperation(drops.clone()),
            PendingOperation(drops.clone()),
            "Timed out",
        ));
        assert_eq!(
            operation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );
        drop(operation);
        assert_eq!(drops.get(), 2);
    }

    thread_local! {
        static REVOKED: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn revoke(url: &str) {
        REVOKED.with(|events| events.borrow_mut().push(url.into()));
    }

    #[test]
    fn cancelling_decoder_releases_its_object_url_once() {
        REVOKED.with(|events| events.borrow_mut().clear());
        let mut decode = Box::pin(async {
            let url = ImageDecodeUrl::new("blob:pending-decode".into(), revoke);
            assert_eq!(url.as_str(), "blob:pending-decode");
            std::future::pending::<()>().await;
        });
        assert_eq!(
            decode
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );
        REVOKED.with(|events| assert!(events.borrow().is_empty()));
        drop(decode);
        REVOKED.with(|events| assert_eq!(*events.borrow(), vec!["blob:pending-decode"]));
    }

    #[test]
    fn decoder_url_releases_after_success_and_error() {
        REVOKED.with(|events| events.borrow_mut().clear());
        for result in [Ok(()), Err("Broken image")] {
            let mut decode = Box::pin(async {
                let _url = ImageDecodeUrl::new("blob:finished-decode".into(), revoke);
                std::future::ready(result).await
            });
            assert_eq!(
                decode
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(result)
            );
            drop(decode);
        }
        REVOKED.with(|events| {
            assert_eq!(
                *events.borrow(),
                vec!["blob:finished-decode", "blob:finished-decode"]
            )
        });
    }

    #[test]
    fn late_image_completion_drops_payload_after_owner_is_cancelled() {
        let drops = Rc::new(Cell::new(0));
        let (owner, receiver) = ImageCompletionOwner::channel();
        let callback = owner.callback();
        drop(owner);
        drop(receiver);
        callback.complete(PendingOperation(drops.clone()));
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn image_completion_delivers_only_the_first_result() {
        let (owner, mut receiver) = ImageCompletionOwner::channel();
        let resolve = owner.callback();
        let reject = owner.callback();
        resolve.complete(Ok::<_, &str>(42));
        reject.complete(Err("late rejection"));
        assert_eq!(receiver.try_recv(), Ok(Some(Ok(42))));
    }
}
