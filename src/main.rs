// A simple GUI calculator built with eframe/egui.
//
// eframe is the application framework (handles the OS window, event loop, etc.).
// egui is the immediate-mode GUI library that draws widgets every frame.
// In immediate-mode GUIs there is no persistent widget state — the UI is
// rebuilt from scratch on every frame based on the application state.

use eframe::{egui, epi};
use rust_calc::utils::format;
//use egui;
//use epi;
//use eframe;


// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// All mutable data that drives the calculator UI.
///
/// Every frame, `update()` reads this state to draw the UI and writes it back
/// when the user presses a button.
struct CalcApp {
    /// The full math expression the user has built so far, shown in the
    /// display.  Tokens are separated by single spaces, e.g. "12 + 7 * 3".
    /// After `=` is pressed this field holds the final result string.
    expression: String,

    /// The number the user is currently typing digit-by-digit.
    /// It has not yet been committed to `expression`.
    current_input: String,

    /// Set to `true` right after `=` is pressed.  When it is `true` and the
    /// user presses a digit, we start a brand-new expression instead of
    /// appending to the old result.
    result_shown: bool,
}

impl Default for CalcApp {
    fn default() -> Self {
        Self {
            expression: String::new(),
            current_input: String::from("0"), // start with a visible zero
            result_shown: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation logic
// ---------------------------------------------------------------------------

/// Evaluate a fully-built expression string such as `"12 + 7 * 3 - 1"`.
///
/// The expression must follow the pattern:
///   `number (operator number)*`
/// where tokens are separated by single spaces.
///
/// Operator precedence is handled with two passes:
///   1. Resolve all `*` and `/` left-to-right.
///   2. Resolve all `+` and `-` left-to-right.
///
/// Returns `Ok(result)` or `Err(message)` (e.g. for division by zero or a
/// malformed expression).
fn evaluate(expression: &str) -> Result<f64, String> {
    // Split the expression into tokens: numbers and operators alternate.
    // Example: "12 + 7 * 3" → ["12", "+", "7", "*", "3"]
    let tokens: Vec<&str> = expression.split_whitespace().collect();

    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }

    // --- First pass: parse all tokens into a flat list while immediately
    // applying `*` and `/`. ---
    //
    // `numbers` accumulates the operands that still need `+` / `-` applied.
    // `ops` accumulates the corresponding `+` / `-` operators between them.
    let mut numbers: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();

    // The first token must be a number.
    let first: f64 = tokens[0]
        .parse()
        .map_err(|_| format!("Cannot parse '{}'", tokens[0]))?;

    // `acc` is the running product/quotient for the current `*`/`/` chain.
    let mut acc = first;

    // Walk the remaining tokens two at a time: operator then operand.
    let mut i = 1;
    while i + 1 <= tokens.len() - 1 {
        let op = tokens[i];
        let num: f64 = tokens[i + 1]
            .parse()
            .map_err(|_| format!("Cannot parse '{}'", tokens[i + 1]))?;

        match op {
            "*" => acc *= num,
            "/" => {
                if num == 0.0 {
                    return Err("Div by zero".to_string());
                }
                acc /= num;
            }
            // For `+` or `-` we commit the current accumulator to the list
            // and start a new accumulator for the next `*`/`/` chain.
            "+" | "-" => {
                numbers.push(acc);
                ops.push(op.chars().next().unwrap());
                acc = num;
            }
            unknown => return Err(format!("Unknown op '{}'", unknown)),
        }

        i += 2; // advance past the operator and the operand we just consumed
    }

    // Push the last accumulated value.
    numbers.push(acc);

    // --- Second pass: apply `+` and `-` left-to-right. ---
    let mut result = numbers[0];
    for (op, &n) in ops.iter().zip(numbers[1..].iter()) {
        match op {
            '+' => result += n,
            '-' => result -= n,
            _ => unreachable!(), // only + and - reach this pass
        }
    }

    Ok(result)
}



// ---------------------------------------------------------------------------
// eframe application impl
// ---------------------------------------------------------------------------

// In eframe 0.17 the application trait is `epi::App`.
impl epi::App for CalcApp {
    /// The window title shown in the OS title bar.
      fn name(&self) -> &str {
        "Calculator"
       
    }

    /// Called by eframe on every frame (typically 60 fps).
    ///
    /// Here we build the entire UI from the current state.  Any button press
    /// mutates `self` and the change is reflected immediately on the next frame.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // The string shown in the display area.
        // While the user is typing, we combine the committed expression with
        // the in-progress digits so the display always shows the full picture.
        let display_text = if self.expression.is_empty() {
            // Nothing committed yet — just show what's being typed.
            self.current_input.clone()
        } else if self.current_input.is_empty() || self.current_input == "0" {
            // An operator was just pressed; show the expression without a
            // trailing zero, e.g. "12 + ".
            format!("{} ", self.expression)
        } else {
            // Mid-expression with some digits already typed.
            format!("{} {}", self.expression, self.current_input)
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Display area ---
            ui.add_space(8.0);

            // Show the current expression in a large heading-size label.
            // A Label is sufficient here — the user never types into the display.
            ui.heading(&display_text);

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // --- Button grid ---
            // We use egui::Grid with 4 columns.  Each cell contains one button.
            egui::Grid::new("calc_grid")
                .num_columns(4)
                .spacing([6.0, 6.0]) // horizontal and vertical gap between cells
                .show(ui, |ui| {
                    // Button size — all buttons share the same dimensions for a
                    // clean, uniform look.
                    let btn_size = egui::vec2(64.0, 48.0);

                    // Helper macro-like inline: creates a button and returns
                    // whether it was clicked this frame.
                    macro_rules! btn {
                        ($label:expr) => {
                            ui.add_sized(btn_size, egui::Button::new($label)).clicked()
                        };
                    }

                    // Row 1 — 7, 8, 9, ÷
                    if btn!("7") { self.press_digit('7'); }
                    if btn!("8") { self.press_digit('8'); }
                    if btn!("9") { self.press_digit('9'); }
                    if btn!("÷") { self.press_operator('/'); }
                    ui.end_row();

                    // Row 2 — 4, 5, 6, ×
                    if btn!("4") { self.press_digit('4'); }
                    if btn!("5") { self.press_digit('5'); }
                    if btn!("6") { self.press_digit('6'); }
                    if btn!("×") { self.press_operator('*'); }
                    ui.end_row();

                    // Row 3 — 1, 2, 3, −
                    if btn!("1") { self.press_digit('1'); }
                    if btn!("2") { self.press_digit('2'); }
                    if btn!("3") { self.press_digit('3'); }
                    if btn!("−") { self.press_operator('-'); }
                    ui.end_row();

                    // Row 4 — 0, C (clear), =, +
                    if btn!("0") { self.press_digit('0'); }
                    if btn!("C") { self.press_clear(); }
                    if btn!("=") { self.press_equals(); }
                    if btn!("+") { self.press_operator('+'); }
                    ui.end_row();
                });
        });
    }
}

// ---------------------------------------------------------------------------
// Button press handlers
// ---------------------------------------------------------------------------

impl CalcApp {
    /// Handle a digit button press (`0`–`9`).
    fn press_digit(&mut self, digit: char) {
        // If the previous action was `=`, start a fresh expression.
        if self.result_shown {
            self.expression.clear();
            self.current_input.clear();
            self.result_shown = false;
        }

        if self.current_input == "0" {
            // Replace the leading zero rather than appending (avoid "007").
            self.current_input = digit.to_string();
        } else {
            self.current_input.push(digit);
        }
    }

    /// Handle an operator button press (`+`, `-`, `*`, `/`).
    fn press_operator(&mut self, op: char) {
        // After `=`, the result is already in `current_input`; allow chaining
        // by continuing from that result.
        self.result_shown = false;

        // Commit current_input to the expression before appending the operator.
        // If the user presses an operator before typing any number, use "0".
        let operand = if self.current_input.is_empty() {
            "0".to_string()
        } else {
            self.current_input.clone()
        };

        if self.expression.is_empty() {
            // First operand — expression was empty.
            self.expression = format!("{} {}", operand, op);
        } else {
            // Append operand and new operator to the existing expression.
            self.expression = format!("{} {} {}", self.expression, operand, op);
        }

        // Reset input so the user can start typing the next number.
        self.current_input.clear();
    }

    /// Handle the `=` button press — evaluate the full expression.
    fn press_equals(&mut self) {
        // Nothing to evaluate if there is no expression and no input.
        if self.expression.is_empty() && self.current_input.is_empty() {
            return;
        }

        // Build the complete expression string to evaluate.
        let full = if self.expression.is_empty() {
            // User pressed = without any operator — just echo the number.
            self.current_input.clone()
        } else {
            // Append the last typed number (or "0" if nothing was typed yet).
            let last = if self.current_input.is_empty() {
                "0".to_string()
            } else {
                self.current_input.clone()
            };
            format!("{} {}", self.expression, last)
        };

        // Evaluate and display the result (or an error message).
        match evaluate(&full) {
            Ok(value) => {
                self.expression.clear();
                self.current_input = format::format_result(value);
            }
            Err(msg) => {
                self.expression.clear();
                self.current_input = msg;
            }
        }

        // Mark that a result is showing so the next digit starts fresh.
        self.result_shown = true;
    }

    /// Handle the `C` (clear) button — reset the calculator to its initial state.
    fn press_clear(&mut self) {
        self.expression.clear();
        self.current_input = String::from("0");
        self.result_shown = false;
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    // Window options — eframe 0.17 uses `NativeOptions` with direct fields.
    let options = eframe::NativeOptions {
        // Set the initial window size in logical pixels.
        initial_window_size: Some(egui::vec2(320.0, 320.0)),
        // Prevent the window from being resized below this.
        min_window_size: Some(egui::vec2(280.0, 280.0)),
        ..Default::default()
    };

    // `run_native` hands control to eframe.  It creates the OS window, starts
    // the event loop, and calls `CalcApp::update` on every frame until the
    // window is closed.  It never returns (diverges).
    eframe::run_native(Box::new(CalcApp::default()), options);
}
