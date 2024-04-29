// Takes a list of delays and a suffix as input, returns a set of textified
// delays
#[must_use]
pub fn text_delays(textify: &[f32], suffix: &str) -> String {
    format!(
        "[{}]",
        textify
            .iter()
            .map(|ds| format!("{ds}{suffix}"))
            .reduce(|acc, text_ds| format!("{acc}, {text_ds}"))
            .unwrap_or_default()
    )
}
