#[cfg(test)]
pub fn init() {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .unwrap_or_default();
}

#[cfg(test)]

mod test {

    use tracing::Level;

    use crate::test_utils::init;

    #[test]
    fn test_tracing() {
        init();

        let span = tracing::span!(Level::INFO, "test_span", sjq = 123);

        let _enter = span.enter();
        tracing::warn!(qq = 33, "trace");
        tracing::error!("error");
        tracing::info!("info");
        tracing::debug!("debug");
    }

    #[test]
    #[ignore]
    fn test_tracing_with_filter() {
        tracing_subscriber::fmt::init();
        let test_span = tracing::span!(Level::INFO, "test_span", sjq = 123);
        test_span.in_scope(|| {
            for i in 0..100 {
                tracing::info!(i);
            }
        });
        let span2 = tracing::span!(Level::INFO, "test_span", sjq = 223);
        span2.in_scope(|| {
            for i in 0..100 {
                tracing::info!(i);
            }
        });
    }
}
