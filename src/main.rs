// A simple GUI calculator built with eframe/egui.
//
// eframe is the application framework (handles the OS window, event loop, etc.).
// egui is the immediate-mode GUI library that draws widgets every frame.
// In immediate-mode GUIs there is no persistent widget state — the UI is
// rebuilt from scratch on every frame based on the application state.

use eframe::{egui, epi};
use rust_calc::utils::format;
use std::sync::mpsc::{self, Receiver};


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

    /// True while the background evaluation thread is running.
    /// The button grid is disabled and a spinner is shown during this time.
    computing: bool,

    /// The receiving end of the channel used to get the result back from the
    /// background thread.  `None` when no computation is in progress.
    result_rx: Option<Receiver<Result<f64, String>>>,
}

impl Default for CalcApp {
    fn default() -> Self {
        Self {
            expression: String::new(),
            current_input: String::from("0"), // start with a visible zero
            result_shown: false,
            computing: false,
            result_rx: None,
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
// Button grid
// ---------------------------------------------------------------------------

/// What happens when a calculator button is pressed.
#[derive(Copy, Clone)]
enum CalcButtonAction {
    Digit(char),
    Operator(char),
    Clear,
    Equals,
}

/// 4×4 button layout: display label and the action it triggers.
const BUTTON_ROWS: &[[(&str, CalcButtonAction); 4]] = &[
    [
        ("7", CalcButtonAction::Digit('7')),
        ("8", CalcButtonAction::Digit('8')),
        ("9", CalcButtonAction::Digit('9')),
        ("÷", CalcButtonAction::Operator('/')),
    ],
    [
        ("4", CalcButtonAction::Digit('4')),
        ("5", CalcButtonAction::Digit('5')),
        ("6", CalcButtonAction::Digit('6')),
        ("×", CalcButtonAction::Operator('*')),
    ],
    [
        ("1", CalcButtonAction::Digit('1')),
        ("2", CalcButtonAction::Digit('2')),
        ("3", CalcButtonAction::Digit('3')),
        ("−", CalcButtonAction::Operator('-')),
    ],
    [
        ("0", CalcButtonAction::Digit('0')),
        ("C", CalcButtonAction::Clear),
        ("=", CalcButtonAction::Equals),
        ("+", CalcButtonAction::Operator('+')),
    ],
];

/// Draw a uniformly sized calculator button; returns `true` if clicked this frame.
fn calc_button(ui: &mut egui::Ui, size: egui::Vec2, label: &str) -> bool {
    ui.add_sized(size, egui::Button::new(label)).clicked()
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
        // --- Poll the background thread result channel ---
        // `try_recv` is non-blocking: it returns immediately with either the
        // result or an error indicating the thread has not finished yet.
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                // Thread finished — apply result and reset computing state.
                match result {
                    Ok(value) => self.current_input = format::format_result(value),
                    Err(msg)  => self.current_input = msg,
                }
                self.expression.clear();
                self.result_shown = true;
                self.computing = false;
                self.result_rx = None;
            }
        }

        // While computing, ask egui to keep repainting every frame so the
        // spinner animation stays smooth even without user input.
        if self.computing {
            ctx.request_repaint();
        }

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

            // Show the expression and, while computing, an animated spinner
            // on the same horizontal line to the right of the text.
            ui.horizontal(|ui| {
                ui.heading(&display_text);
                if self.computing {
                    // `egui::Spinner` is a built-in widget that draws a
                    // rotating arc — no extra dependencies needed.
                    ui.add(egui::Spinner::new());
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // --- Button grid ---
            // Wrap the grid in `add_enabled_ui` so all buttons are visually
            // greyed out and non-interactive while computing.
            ui.add_enabled_ui(!self.computing, |ui| {
                // We use egui::Grid with 4 columns.  Each cell contains one button.
                egui::Grid::new("calc_grid")
                    .num_columns(4)
                    .spacing([6.0, 6.0]) // horizontal and vertical gap between cells
                    .show(ui, |ui| {
                        let btn_size = egui::vec2(64.0, 48.0);

                        for row in BUTTON_ROWS {
                            for (label, action) in row {
                                if calc_button(ui, btn_size, label) {
                                    self.dispatch(*action);
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Button press handlers
// ---------------------------------------------------------------------------

impl CalcApp {
    fn dispatch(&mut self, action: CalcButtonAction) {
        match action {
            CalcButtonAction::Digit(d) => self.press_digit(d),
            CalcButtonAction::Operator(op) => self.press_operator(op),
            CalcButtonAction::Clear => self.press_clear(),
            CalcButtonAction::Equals => self.press_equals(),
        }
    }

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

    /// Handle the `=` button press — kick off evaluation on a background thread.
    fn press_equals(&mut self) {
        // Nothing to evaluate if there is no expression and no input.
        if self.expression.is_empty() && self.current_input.is_empty() {
            return;
        }

        // Ignore repeated presses while a computation is already running.
        if self.computing {
            return;
        }

        // Build the complete expression string to hand to the worker thread.
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

        // Create a one-shot channel.  The worker sends exactly one message.
        let (tx, rx) = mpsc::channel();
        self.result_rx = Some(rx);
        self.computing = true;

        // Spawn the worker thread.  It sleeps for 2 s so the spinner is
        // visible, then evaluates the expression and sends the result back.
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let result = evaluate(&full);
            // If the receiver was dropped (e.g. user pressed C), this is a
            // no-op — the send error is intentionally ignored.
            let _ = tx.send(result);
        });
    }

    /// Handle the `C` (clear) button — reset the calculator to its initial state.
    ///
    /// Dropping `result_rx` causes the background thread's `send()` to fail
    /// silently, so the thread still runs to completion but its result is
    /// discarded.
    fn press_clear(&mut self) {
        self.expression.clear();
        self.current_input = String::from("0");
        self.result_shown = false;
        self.computing = false;
        self.result_rx = None; // dropping the Receiver cancels the pending result
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
