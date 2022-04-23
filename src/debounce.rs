use futures::stream::Stream;
use futures::FutureExt;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;
use tokio::time::{sleep, Instant, Sleep};

#[pin_project]
pub struct DebounceTimeFilter<S: Stream> {
    duration: Duration,
    #[pin]
    stream: Option<S>,
    last_value: Option<S::Item>,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for DebounceTimeFilter<S> {
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let delay = this.delay;
        let last_value = this.last_value;
        let mut stream = this.stream;

        let mut poll_res = Poll::Pending;

        let buf_is_empty = last_value.is_none();
        // 挂起的值到时间就返回
        if !buf_is_empty && delay.poll_unpin(cx).is_ready() {
            poll_res = Poll::Ready(last_value.take());
        }

        let mut stream_is_terminated = stream.is_none();
        if let Some(pin_stream) = stream.as_mut().as_pin_mut() {
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
                    stream.set(None);
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

impl<S: Stream> DebounceTimeFilter<S> {
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
pub struct DebounceTime<S: Stream> {
    duration: Duration,
    #[pin]
    stream: Option<S>,
    last_value: Option<S::Item>,
    delay: Pin<Box<Sleep>>,
}

impl<S: Stream> Stream for DebounceTime<S> {
    type Item = Result<S::Item, Debounced<S::Item>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let duration = self.duration;
        let this = self.project();
        let delay = this.delay;
        let last_value = this.last_value;
        let mut stream = this.stream;

        let mut poll_res = Poll::Pending;

        let buf_is_empty = last_value.is_none();
        // 挂起的值到时间就返回
        if !buf_is_empty && delay.poll_unpin(cx).is_ready() {
            poll_res = Poll::Ready(last_value.take().map(Ok));
        }

        let mut stream_is_terminated = stream.is_none();
        if let Some(pin_stream) = stream.as_mut().as_pin_mut() {
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
                    stream.set(None);
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

impl<S: Stream> DebounceTime<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream: Some(stream),
            delay: Box::pin(sleep(Duration::from_nanos(0))),
            last_value: None,
            duration,
        }
    }
}

#[pin_project]
pub struct DebounceFilter<S: Stream, Selector, Fut> {
    selector: Selector,
    #[pin]
    debouncer: Option<Fut>,
    #[pin]
    stream: Option<S>,
    last_value: Option<S::Item>,
}

impl<S: Stream, Selector, Fut> Stream for DebounceFilter<S, Selector, Fut>
where
    Selector: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut debouncer: Pin<&mut Option<Fut>> = this.debouncer;
        let last_value = this.last_value;
        let mut stream: Pin<&mut Option<S>> = this.stream;
        let selector = this.selector;

        let mut poll_res = Poll::Pending;

        let buf_is_empty = last_value.is_none();
        if !buf_is_empty {
            if let Some(debouncer) = debouncer.as_mut().as_pin_mut() {
                if debouncer.poll(cx).is_ready() {
                    poll_res = Poll::Ready(last_value.take());
                }
            }
        }

        let mut stream_is_terminated = stream.is_none();
        if let Some(pin_stream) = stream.as_mut().as_pin_mut() {
            // 从stream中获取值，替换掉挂起的值
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    debouncer.set(Some(selector(&value)));
                    *last_value = Some(value);
                    cx.waker().wake_by_ref();
                }
                Poll::Ready(None) => {
                    stream.set(None);
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

impl<S: Stream, Selector, Fut> DebounceFilter<S, Selector, Fut> {
    pub fn new(stream: S, selector: Selector) -> Self {
        Self {
            stream: Some(stream),
            debouncer: None,
            last_value: None,
            selector,
        }
    }
}

#[pin_project]
pub struct Debounce<S: Stream, Selector, Fut> {
    selector: Selector,
    #[pin]
    debouncer: Option<Fut>,
    #[pin]
    stream: Option<S>,
    last_value: Option<S::Item>,
}

impl<S: Stream, Selector, Fut> Stream for Debounce<S, Selector, Fut>
where
    Selector: FnMut(&S::Item) -> Fut,
    Fut: Future,
{
    type Item = Result<S::Item, Debounced<S::Item>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut debouncer = this.debouncer;
        let last_value = this.last_value;
        let mut stream = this.stream;
        let selector = this.selector;

        let mut poll_res = Poll::Pending;

        let buf_is_empty = last_value.is_none();
        if !buf_is_empty {
            if let Some(debouncer) = debouncer.as_mut().as_pin_mut() {
                if debouncer.poll(cx).is_ready() {
                    poll_res = Poll::Ready(last_value.take().map(Ok));
                }
            }
        }
        let mut stream_is_terminated = stream.is_none();
        if let Some(pin_stream) = stream.as_mut().as_pin_mut() {
            match pin_stream.poll_next(cx) {
                Poll::Ready(Some(value)) => {
                    debouncer.set(Some(selector(&value)));
                    if let Some(debounced) = last_value.replace(value) {
                        poll_res = Poll::Ready(Some(Err(Debounced(debounced))));
                    } else {
                        cx.waker().wake_by_ref();
                    }
                }
                Poll::Ready(None) => {
                    stream.set(None);
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

impl<S: Stream, Selector, Fut> Debounce<S, Selector, Fut> {
    pub fn new(stream: S, selector: Selector) -> Self {
        Self {
            stream: Some(stream),
            debouncer: None,
            last_value: None,
            selector,
        }
    }
}
