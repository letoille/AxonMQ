use chrono::DateTime;
use minijinja::Environment;

pub struct MinijinjaFilter;

impl MinijinjaFilter {
    pub fn register(env: &mut Environment<'static>) {
        env.add_filter("date", Self::date);
    }

    fn date(ts_ms: i64, format: Option<String>) -> String {
        let format_str = format.unwrap_or("%Y-%m-%d %H:%M:%S".to_string());

        if let Some(dt) = DateTime::from_timestamp_millis(ts_ms) {
            dt.format(&format_str).to_string()
        } else {
            "".to_string()
        }
    }
}
