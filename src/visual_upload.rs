//! ADR 0025: application-owned upload source, separately bounded from response SSE.
//! Consumed means handed to the HTTP transport, not provider receipt or erasure.
use crate::cancellation::Cancellation;
use futures_util::Stream;
use std::{
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::time::Instant;

struct State {
    bytes: Option<Vec<u8>>,
    consumed: bool,
    closed: bool,
}
pub(crate) struct Upload {
    state: Arc<Mutex<State>>,
    deadline: Instant,
}
struct Source {
    state: Arc<Mutex<State>>,
    deadline: Instant,
    cancel: Cancellation,
    ended: bool,
}
impl Stream for Source {
    type Item = Result<Vec<u8>, io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.ended {
            return Poll::Ready(None);
        }
        let result = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if !state.consumed && (Instant::now() >= self.deadline || self.cancel.is_cancelled()) {
                state.bytes.take();
                state.closed = true;
            }
            if let Some(bytes) = state.bytes.take() {
                state.consumed = true;
                Some(Ok(bytes))
            } else if state.consumed {
                None
            } else {
                Some(Err(io::Error::other("visual upload source closed")))
            }
        };
        self.ended = true;
        Poll::Ready(result)
    }
}
impl Upload {
    pub(crate) fn new(bytes: Vec<u8>, deadline: Instant) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                bytes: Some(bytes),
                consumed: false,
                closed: false,
            })),
            deadline,
        }
    }
    fn source(&self, cancel: &Cancellation) -> Source {
        Source {
            state: self.state.clone(),
            deadline: self.deadline,
            cancel: cancel.clone(),
            ended: false,
        }
    }
    pub(crate) fn body(&self, cancel: &Cancellation) -> reqwest::Body {
        reqwest::Body::wrap_stream(self.source(cancel))
    }
    /// Called only after successful SSE headers. An early header alone cannot
    /// extend the image deadline while the application still owns pending bytes.
    pub(crate) fn admit_response(&self, cancel: &Cancellation) -> Result<(), &'static str> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if cancel.is_cancelled() {
            return Err("visual upload cancelled; remote outcome unknown");
        }
        if Instant::now() >= self.deadline {
            return Err("visual upload deadline; remote outcome unknown");
        }
        if state.closed || !state.consumed || state.bytes.is_some() {
            return Err("visual upload source not consumed; remote outcome unknown");
        }
        Ok(())
    }
    pub(crate) fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.bytes.take();
        state.closed = true;
    }
}
impl Drop for Upload {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    #[tokio::test(start_paused = true)]
    async fn early_headers_expiry_and_cancel_cannot_admit_pending_source() {
        let cancel = Cancellation::default();
        let upload = Upload::new(
            vec![7; 1024],
            Instant::now() + std::time::Duration::from_secs(10),
        );
        assert!(upload.admit_response(&cancel).is_err());
        let mut source = upload.source(&cancel);
        assert_eq!(source.next().await.unwrap().unwrap().len(), 1024);
        assert!(upload.state.lock().unwrap().bytes.is_none());
        assert!(upload.admit_response(&cancel).is_ok());
        cancel.cancel();
        assert!(upload.admit_response(&cancel).is_err());
        tokio::time::advance(std::time::Duration::from_secs(10)).await;
        assert!(upload.admit_response(&Cancellation::default()).is_err());
    }
    #[tokio::test]
    async fn dropping_upload_clears_pending_bytes_even_when_transport_retains_source() {
        let upload = Upload::new(
            vec![9; 1024],
            Instant::now() + std::time::Duration::from_secs(10),
        );
        let state = upload.state.clone();
        let mut source = upload.source(&Cancellation::default());
        drop(upload);
        assert!(state.lock().unwrap().bytes.is_none());
        assert!(source.next().await.unwrap().is_err());
        assert!(source.next().await.is_none());
    }
    #[tokio::test(start_paused = true)]
    async fn expired_or_cancelled_transport_poll_cannot_yield_pending_image_bytes() {
        for expired in [false, true] {
            let cancel = Cancellation::default();
            let upload = Upload::new(
                vec![3; 1024],
                Instant::now() + std::time::Duration::from_secs(10),
            );
            let mut source = upload.source(&cancel);
            if expired {
                tokio::time::advance(std::time::Duration::from_secs(10)).await;
            } else {
                cancel.cancel();
            }
            assert!(source.next().await.unwrap().is_err());
            assert!(upload.state.lock().unwrap().bytes.is_none());
            assert!(!upload.state.lock().unwrap().consumed);
            assert!(upload.admit_response(&cancel).is_err());
        }
    }
}
