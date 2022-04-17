use futures::stream::Stream;
use futures::FutureExt;
use pin_project::pin_project;
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

        let buf_is_empty = last_value.is_none();
        // 挂起的值到时间就返回
        if !buf_is_empty && delay.poll_unpin(cx).is_ready() {
            poll_res = Poll::Ready(last_value.take());
        }

        let mut stream_is_terminated = stream.is_none();
        if let Some(s) = stream {
            let pin_stream = unsafe { Pin::new_unchecked(s) };
            // 从stream中获取值，替换掉挂起的值
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    delay.as_mut().reset(Instant::now() + duration);
                    *last_value = Some(value);
                    // poll_res.is_pending() <-
                    //     buf_is_empty ||
                    //     delay.poll_unpin(cx).is_pending()
                    // 第二种情况这个stream会被唤醒，只需要buf_is_empty辅助需要唤醒
                    if buf_is_empty {
                        cx.waker().wake_by_ref();
                    }
                }
                Poll::Ready(None) => {
                    *stream = None;
                    stream_is_terminated = true;
                }
                Poll::Pending => {}
            }
        }

        // stream结束且没有挂起的值
        if buf_is_empty && stream_is_terminated {
            poll_res = Poll::Ready(None);
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

        let buf_is_empty = last_value.is_none();
        // 挂起的值到时间就返回
        if !buf_is_empty && delay.poll_unpin(cx).is_ready() {
            poll_res = Poll::Ready(last_value.take().map(Ok));
        }

        let mut stream_is_terminated = stream.is_none();
        if let Some(s) = stream {
            let pin_stream = unsafe { Pin::new_unchecked(s) };
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    delay.as_mut().reset(Instant::now() + duration);
                    if let Some(debounced) = last_value.replace(value) {
                        poll_res = Poll::Ready(Some(Err(Debounced(debounced))));
                    } else if buf_is_empty {
                        cx.waker().wake_by_ref();
                    }
                }
                Poll::Ready(None) => {
                    *stream = None;
                    stream_is_terminated = true;
                }
                Poll::Pending => {}
            }
        }

        // stream结束且没有挂起的值
        if buf_is_empty && stream_is_terminated {
            poll_res = Poll::Ready(None);
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
