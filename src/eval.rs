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
pub fn evaluate(expression: &str) -> Result<f64, String> {
    let tokens: Vec<&str> = expression.split_whitespace().collect();

    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut numbers: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();

    let first: f64 = tokens[0]
        .parse()
        .map_err(|_| format!("Cannot parse '{}'", tokens[0]))?;

    let mut acc = first;

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
            "+" | "-" => {
                numbers.push(acc);
                ops.push(op.chars().next().unwrap());
                acc = num;
            }
            unknown => return Err(format!("Unknown op '{}'", unknown)),
        }

        i += 2;
    }

    numbers.push(acc);

    let mut result = numbers[0];
    for (op, &n) in ops.iter().zip(numbers[1..].iter()) {
        match op {
            '+' => result += n,
            '-' => result -= n,
            _ => unreachable!(),
        }
    }

    Ok(result)
}
