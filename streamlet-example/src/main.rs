use async_stream::stream;
use chunk_debounce::chunk_debounce;
use futures::{Stream, StreamExt};
use std::time::Duration;
use streamlet::Streamlet;

mod chunk_debounce;

#[tokio::main]
async fn main() {
    {
        let stream = a_stream();
        stream
            .debounce_time(Duration::from_millis(200))
            // .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .debounce_time_filter(Duration::from_millis(200))
            .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .throttle_time(Duration::from_millis(200))
            // .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .throttle_time_filter(Duration::from_millis(200))
            .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let tx = chunk_debounce(
            Duration::from_millis(200),
            futures::future::pending::<()>(),
            |msg, debounceds| async move {
                println!("debounceds {:?}, msg {:?}", debounceds, msg);
            },
        );

        let mut stream = a_stream().boxed();
        while let Some(msg) = stream.next().await {
            let _ = tx.unbounded_send(msg);
        }
    }
}

fn a_stream() -> impl Stream<Item = i32> {
    stream! {
        for i in 0..5 {
            yield i;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        for i in 5..10 {
            yield i;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }
}
