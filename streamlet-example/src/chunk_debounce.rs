use futures::channel::mpsc::UnboundedSender;
use futures::stream::StreamExt;
use std::future::Future;
use std::mem;
use std::time::Duration;
use streamlet::debounce::Debounced;
use streamlet::Streamlet;

pub fn chunk_debounce<Subscriber, Fut, Until, Msg>(
    duration: Duration,
    until: Until,
    mut subscriber: Subscriber,
) -> UnboundedSender<Msg>
where
    Msg: Send + 'static,
    Fut: Future<Output = ()> + Send,
    Until: Future<Output = ()> + Send + 'static,
    Subscriber: FnMut(Msg, Vec<Debounced<Msg>>) -> Fut + Send + 'static,
{
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut acc = vec![];
        let debounce_rx = rx
            .debounce_time(duration)
            .take_until(until)
            .filter_map(move |msg| {
                let next = match msg {
                    Ok(msg) => {
                        let debounceds = mem::replace(&mut acc, vec![]);
                        Some((msg, debounceds))
                    }
                    Err(debounced) => {
                        acc.push(debounced);
                        None
                    }
                };
                futures::future::ready(next)
            });

        debounce_rx
            .for_each_concurrent(None, |(msg, debounced)| subscriber(msg, debounced))
            .await
    });

    tx
}
