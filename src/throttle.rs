use futures::{FutureExt, Stream};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{sleep, Instant, Sleep};

#[pin_project]
pub struct ThrottleTimeFilter<S> {
    #[pin]
    stream: S,
    duration: Duration,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for ThrottleTimeFilter<S> {
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

impl<S> ThrottleTimeFilter<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream,
            duration,
            delay: Box::pin(sleep(Duration::from_nanos(0))),
        }
    }
}

#[pin_project]
pub struct ThrottleTime<S> {
    #[pin]
    stream: S,
    duration: Duration,
    last_time: Option<Instant>,
}

#[derive(Debug, Copy, Clone)]
pub struct Throttled<T>(pub T);

impl<S: Stream> Stream for ThrottleTime<S> {
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

impl<S> ThrottleTime<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream,
            duration,
            last_time: None,
        }
    }
}

#[pin_project]
pub struct ThrottleFilter<S, Selector, Fut> {
    #[pin]
    stream: S,
    selector: Selector,
    #[pin]
    delay: Option<Fut>,
}

impl<S: Stream, Selector, Fut> Stream for ThrottleFilter<S, Selector, Fut>
where
    Selector: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let stream = this.stream;
        let selector = this.selector;
        let mut delay = this.delay;

        let select = if let Some(delay) = delay.as_mut().as_pin_mut() {
            delay.poll(cx).is_ready()
        } else {
            true
        };

        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if select {
                    delay.set(Some(selector(&value)));
                    Poll::Ready(Some(value))
                } else {
                    Poll::Pending
                }
            }
            pending_or_done => {
                if select {
                    delay.set(None);
                }
                pending_or_done
            }
        }
    }
}

impl<S, Selector, Fut> ThrottleFilter<S, Selector, Fut> {
    pub fn new(stream: S, selector: Selector) -> Self {
        Self {
            stream,
            selector,
            delay: None,
        }
    }
}

#[pin_project]
pub struct Throttle<S, Selector, Fut> {
    #[pin]
    stream: S,
    selector: Selector,
    #[pin]
    delay: Option<Fut>,
}

impl<S: Stream, Selector, Fut> Stream for Throttle<S, Selector, Fut>
where
    Selector: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    type Item = Result<S::Item, Throttled<S::Item>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let stream = this.stream;
        let selector = this.selector;
        let mut delay = this.delay;

        let select = if let Some(delay) = delay.as_mut().as_pin_mut() {
            delay.poll(cx).is_ready()
        } else {
            true
        };

        match stream.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if select {
                    delay.set(Some(selector(&value)));
                    Poll::Ready(Some(Ok(value)))
                } else {
                    Poll::Ready(Some(Err(Throttled(value))))
                }
            }
            pending_or_done => {
                if select {
                    delay.set(None);
                }
                pending_or_done.map(|v| v.map(Ok))
            }
        }
    }
}

impl<S, Selector, Fut> Throttle<S, Selector, Fut> {
    pub fn new(stream: S, selector: Selector) -> Self {
        Self {
            stream,
            selector,
            delay: None,
        }
    }
}
