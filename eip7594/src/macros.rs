#[macro_export]
macro_rules! with_optional_threadpool {
    ($self:expr, $body:expr) => {{
        #[cfg(feature = "multithreaded")]
        {
            $self.thread_pool.install(|| $body)
        }
        #[cfg(not(feature = "multithreaded"))]
        {
            $body
        }
    }};
}
