use futures::{FutureExt, Stream};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{sleep, Instant, Sleep};

#[pin_project]
pub struct ThrottleFilter<S> {
    #[pin]
    stream: S,
    duration: Duration,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for ThrottleFilter<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let stream: Pin<&mut S> = this.stream;
        let mut delay: Pin<&mut Sleep> = this.delay.as_mut();

        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if delay.poll_unpin(cx).is_ready() {
                    delay.reset((Instant::now() + duration).into());
                    Poll::Ready(Some(value))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> ThrottleFilter<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream,
            duration,
            delay: Box::pin(sleep(Duration::from_nanos(0))),
        }
    }
}

#[pin_project]
pub struct Throttle<S> {
    #[pin]
    stream: S,
    duration: Duration,
    last_time: Option<Instant>,
}

#[derive(Debug, Copy, Clone)]
pub struct Throttled<T>(pub T);

impl<S: Stream> Stream for Throttle<S> {
    type Item = Result<S::Item, Throttled<S::Item>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let stream: Pin<&mut S> = this.stream;
        let last_time: &mut Option<Instant> = this.last_time;

        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if last_time
                    .as_ref()
                    .map(Instant::elapsed)
                    .map(|i| i > duration)
                    .unwrap_or(true)
                {
                    *last_time = Some(Instant::now());
                    Poll::Ready(Some(Ok(value)))
                } else {
                    Poll::Ready(Some(Err(Throttled(value))))
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> Throttle<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream,
            duration,
            last_time: None,
        }
    }
}
