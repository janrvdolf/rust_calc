use crate::utils::format_new::format_new;

/// Format a floating-point result for display.
///
/// If the value is a whole number (e.g. 6.0), show it without a decimal point.
/// Otherwise show up to 10 significant digits, trimming trailing zeros.
pub fn format_result(value: f64) -> String {
    format_new(value)
}