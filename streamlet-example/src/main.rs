use async_stream::stream;
use streamlet::Streamlet;
use futures::{Stream, StreamExt};
use std::time::Duration;

#[tokio::main]
async fn main() {
    {
        let stream = a_stream();
        stream
            .debounce(Duration::from_millis(200))
            // .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .debounce_filter(Duration::from_millis(200))
            .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .throttle(Duration::from_millis(200))
            // .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
    }

    {
        let stream = a_stream();
        stream
            .throttle_filter(Duration::from_millis(200))
            .map(|x| x * x)
            .for_each(|x| async move {
                println!("{:?}", x)
            }).await;
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