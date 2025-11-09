pub fn now_milliseconds() -> u64 {
    coarsetime::Clock::now_since_epoch().as_millis()
}
