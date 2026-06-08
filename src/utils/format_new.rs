/// Format a floating-point result for display.
///
/// If the value is a whole number (e.g. 6.0), show it without a decimal point.
/// Otherwise show up to 10 significant digits, trimming trailing zeros.
pub fn format_new(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        // Cast is safe here: we verified fract() == 0 and magnitude is finite
        format!("{}", value as i64)
    } else {
        // Remove trailing zeros after the decimal point for readability
        let s = format!("{:.10}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}