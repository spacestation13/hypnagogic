// Takes a list of delays and a suffix as input, returns a set of textified delays
pub fn text_delays (textify :&Vec<f32>, suffix: &str) -> String {
    format!(
        "[{}]",
        textify
            .into_iter()
            .map(|ds| format!("{ds}{suffix}"))
            .reduce(|acc, text_ds| format!("{acc}, {text_ds}"))
            .unwrap_or_default()
    )
}
