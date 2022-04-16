use futures::stream::Stream;
use futures::FutureExt;
use pin_project::pin_project;
use std::mem;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;
use tokio::time::{sleep, Instant, Sleep};

#[pin_project]
pub struct DebounceFilter<S: Stream> {
    duration: Duration,
    stream: Option<S>,
    last_value: Option<S::Item>,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for DebounceFilter<S> {
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let delay = this.delay;
        let last_value = this.last_value;
        let stream = this.stream;

        let mut poll_res = Poll::Pending;
        // 0b000: stream 没有结束，且还有挂起的值
        // 0b001: stream 没有结束，但没有挂起的值
        // 0b010: stream 刚结束，且还有挂起的值
        // 0b011: stream 刚结束，但没有挂起的值
        // 0b100: stream 已经结束，且还有挂起的值
        // 0b101: stream 已经结束，但没有挂起的值
        let mut end_flag: u8 = last_value.is_none() as u8;
        let mut state_changed = false;

        if last_value.is_some() && delay.poll_unpin(cx).is_ready() {
            delay.as_mut().reset(Instant::now() + duration);
            poll_res = Poll::Ready(last_value.take());
        }

        if let Some(s) = stream {
            let pin_stream = unsafe { Pin::new_unchecked(s) };
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    delay.as_mut().reset(Instant::now() + duration);
                    *last_value = Some(value);
                    state_changed = true;
                }
                Poll::Ready(None) => {
                    end_flag |= 0b010;
                }
                Poll::Pending => {}
            }
        } else {
            end_flag |= 0b100;
        }

        // stream结束且没有挂起的值
        if end_flag == 0b011 || end_flag == 0b101 {
            poll_res = Poll::Ready(None);
        }

        // stream刚结束，且有挂起的值
        if end_flag == 0b010 {
            *stream = None;
            state_changed = true;
        }

        if state_changed && poll_res.is_pending() {
            cx.waker().wake_by_ref();
        }

        poll_res
    }
}

impl<S: Stream> DebounceFilter<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream: Some(stream),
            delay: Box::pin(sleep(Duration::from_nanos(0))),
            last_value: None,
            duration,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Debounced<T>(pub T);

#[pin_project]
pub struct Debounce<S: Stream> {
    duration: Duration,
    stream: Option<S>,
    last_value: Option<S::Item>,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for Debounce<S> {
    type Item = Result<S::Item, Debounced<S::Item>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let delay = this.delay;
        let last_value = this.last_value;
        let stream = this.stream;

        let mut poll_res = Poll::Pending;
        // 0b000: stream 没有结束，且还有挂起的值
        // 0b001: stream 没有结束，但没有挂起的值
        // 0b010: stream 刚结束，且还有挂起的值
        // 0b011: stream 刚结束，但没有挂起的值
        // 0b100: stream 已经结束，且还有挂起的值
        // 0b101: stream 已经结束，但没有挂起的值
        let mut end_flag: u8 = last_value.is_none() as u8;
        let mut state_changed = false;

        if last_value.is_some() && delay.poll_unpin(cx).is_ready() {
            delay.as_mut().reset(Instant::now() + duration);
            poll_res = Poll::Ready(last_value.take().map(Ok));
        }

        if let Some(s) = stream {
            let pin_stream = unsafe { Pin::new_unchecked(s) };
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    delay.as_mut().reset(Instant::now() + duration);
                    state_changed = true;
                    if let Some(debounced) = mem::replace(last_value, Some(value)) {
                        poll_res = Poll::Ready(Some(Err(Debounced(debounced))));
                    }
                }
                Poll::Ready(None) => {
                    end_flag |= 0b010;
                }
                Poll::Pending => {}
            }
        } else {
            end_flag |= 0b100;
        }

        // stream结束且没有挂起的值
        if end_flag == 0b011 || end_flag == 0b101 {
            poll_res = Poll::Ready(None);
        }

        // stream刚结束，且有挂起的值
        if end_flag == 0b010 {
            *stream = None;
            state_changed = true;
        }

        if state_changed && poll_res.is_pending() {
            cx.waker().wake_by_ref();
        }

        poll_res
    }
}

impl<S: Stream> Debounce<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream: Some(stream),
            delay: Box::pin(sleep(Duration::from_nanos(0))),
            last_value: None,
            duration,
        }
    }
}
