use futures::Stream;
use std::time::Duration;

pub mod debounce;
pub mod throttle;

pub trait Streamlet {
    fn debounce_time(self, duration: Duration) -> debounce::DebounceTime<Self>
    where
        Self: Sized + Stream,
    {
        debounce::DebounceTime::new(self, duration)
    }

    fn debounce_time_filter(self, duration: Duration) -> debounce::DebounceTimeFilter<Self>
    where
        Self: Sized + Stream,
    {
        debounce::DebounceTimeFilter::new(self, duration)
    }

    fn debounce<Selector, Fut>(self, selector: Selector) -> debounce::Debounce<Self, Selector, Fut>
    where
        Self: Sized + Stream,
    {
        debounce::Debounce::new(self, selector)
    }

    fn debounce_filter<Selector, Fut>(
        self,
        selector: Selector,
    ) -> debounce::DebounceFilter<Self, Selector, Fut>
    where
        Self: Sized + Stream,
    {
        debounce::DebounceFilter::new(self, selector)
    }

    fn throttle_time(self, duration: Duration) -> throttle::ThrottleTime<Self>
    where
        Self: Sized,
    {
        throttle::ThrottleTime::new(self, duration)
    }

    fn throttle_time_filter(self, duration: Duration) -> throttle::ThrottleTimeFilter<Self>
    where
        Self: Sized,
    {
        throttle::ThrottleTimeFilter::new(self, duration)
    }

    fn throttle<Selector, Fut>(self, selector: Selector) -> throttle::Throttle<Self, Selector, Fut>
    where
        Self: Sized,
    {
        throttle::Throttle::new(self, selector)
    }

    fn throttle_filter<Selector, Fut>(
        self,
        selector: Selector,
    ) -> throttle::ThrottleFilter<Self, Selector, Fut>
    where
        Self: Sized,
    {
        throttle::ThrottleFilter::new(self, selector)
    }
}

impl<S: Stream> Streamlet for S {}
