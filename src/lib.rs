use std::time::Duration;
use futures::Stream;

pub mod debounce;
pub mod throttle;

pub trait Streamlet {
    fn debounce(self, duration: Duration) -> debounce::Debounce<Self>
    where
        Self: Sized + Stream,
    {
        debounce::Debounce::new(self, duration)
    }

    fn debounce_filter(self, duration: Duration) -> debounce::DebounceFilter<Self>
    where
        Self: Sized + Stream,
    {
        debounce::DebounceFilter::new(self, duration)
    }

    fn throttle(self, duration: Duration) -> throttle::Throttle<Self>
    where
        Self: Sized,
    {
        throttle::Throttle::new(self, duration)
    }

    fn throttle_filter(self, duration: Duration) -> throttle::ThrottleFilter<Self>
    where
        Self: Sized,
    {
        throttle::ThrottleFilter::new(self, duration)
    }
}

impl<S: Stream> Streamlet for S {}

