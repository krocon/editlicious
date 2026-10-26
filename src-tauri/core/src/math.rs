//! Arithmetic in the text (prompt 020): a small parser for expressions like `255/8`,
//! `2 sin(pi/4)`, `convert(3, 4, 100)` – after Marc's Java MathParser (`java-k-editor/MathParser`):
//! the same functions and constants, implicit multiplication and the tidying of the `Arranger`,
//! but with a real grammar (`^` right-associative, unary minus below `^`) and `fac` that does not
//! overflow into nonsense.
//!
//! ```text
//! expr    := term (('+' | '-') term)*
//! term    := unary (('*' | '/' | '%') unary | implicit unary)*
//! unary   := ('-' | '+') unary | power
//! power   := postfix ('^' unary)?
//! postfix := primary '!'*
//! primary := number | name | name '(' args ')' | '(' expr ')'
//! ```

use std::collections::HashMap;

/// Longest expression looked at, nesting at most.
pub const MAX_LEN: usize = 1000;
const MAX_DEPTH: usize = 64;

/// Functions of one value (as in the template's `OneValueCalculationFactory`) plus `ln`.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "radians", "rad",
    "degrees", "exp", "expm1", "log", "ln", "log10", "log1p", "sqrt", "cbrt", "ceil", "floor",
    "rint", "round", "abs", "ulp", "signum", "fac",
];

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Fac(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// Value and the text as written (for the decimal comma).
    Num(f64, String),
    Name(String),
    Op(char),
    Open,
    Close,
    Comma,
    Bang,
}

fn lex(s: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    // a decimal comma only between digits and outside function arguments – decided by the parser
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => i += 1,
            '0'..='9' | '.' => {
                let start = i;
                let radix = if c == '0' && i + 1 < chars.len() {
                    match chars[i + 1] {
                        'x' | 'X' => 16,
                        'b' | 'B' => 2,
                        _ => 10,
                    }
                } else {
                    10
                };
                if radix != 10 {
                    i += 2;
                    let ds = i;
                    while i < chars.len() && (chars[i].is_digit(radix) || chars[i] == '_') {
                        i += 1;
                    }
                    let digits: String = chars[ds..i].iter().filter(|&&c| c != '_').collect();
                    let v = u64::from_str_radix(&digits, radix).ok()?;
                    out.push(Tok::Num(v as f64, "0x".into()));
                    continue;
                }
                while i < chars.len()
                    && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_')
                {
                    i += 1;
                }
                // exponent
                if i < chars.len()
                    && (chars[i] == 'e' || chars[i] == 'E')
                    && chars
                        .get(i + 1)
                        .is_some_and(|d| d.is_ascii_digit() || *d == '-' || *d == '+')
                    && chars
                        .get(i + if chars[i + 1].is_ascii_digit() { 1 } else { 2 })
                        .is_some_and(|d| d.is_ascii_digit())
                {
                    i += 2;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let text: String = chars[start..i].iter().filter(|&&c| c != '_').collect();
                if text.matches('.').count() > 1 {
                    return None; // a version number 1.2.3
                }
                out.push(Tok::Num(text.parse().ok()?, text));
            }
            c if c.is_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Name(chars[start..i].iter().collect()));
            }
            '+' | '-' | '*' | '/' | '^' | '%' => {
                out.push(Tok::Op(c));
                i += 1;
            }
            '×' | '·' => {
                out.push(Tok::Op('*'));
                i += 1;
            }
            '÷' => {
                out.push(Tok::Op('/'));
                i += 1;
            }
            '−' => {
                out.push(Tok::Op('-'));
                i += 1;
            }
            '(' => {
                out.push(Tok::Open);
                i += 1;
            }
            ')' => {
                out.push(Tok::Close);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '!' => {
                out.push(Tok::Bang);
                i += 1;
            }
            _ => return None,
        }
    }
    Some(out)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    at: usize,
    depth: usize,
    /// Inside function arguments: a comma separates, it is no decimal comma.
    args: usize,
    vars: &'a HashMap<String, f64>,
    /// Facts for the "is this arithmetic?" test.
    binary_ops: usize,
    calls: usize,
    numbers: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        if t.is_some() {
            self.at += 1;
        }
        t
    }

    fn expr(&mut self) -> Option<Expr> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut left = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.at += 1;
            let right = self.term()?;
            self.binary_ops += 1;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        self.depth -= 1;
        Some(left)
    }

    fn term(&mut self) -> Option<Expr> {
        let mut left = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op(op @ ('*' | '/' | '%'))) => {
                    self.at += 1;
                    let right = self.unary()?;
                    self.binary_ops += 1;
                    left = Expr::Bin(op, Box::new(left), Box::new(right));
                }
                // implicit multiplication: 3x, 2 sin(x), (1+2)(3), (2+1) 5 – but never two
                // numbers in a row (a phone number "0617 4123" is no product)
                Some(Tok::Name(_)) | Some(Tok::Open) => {
                    let right = self.unary()?;
                    left = Expr::Bin('*', Box::new(left), Box::new(right));
                    self.binary_ops += 1;
                }
                Some(Tok::Num(..)) if self.prev_close() => {
                    let right = self.unary()?;
                    left = Expr::Bin('*', Box::new(left), Box::new(right));
                    self.binary_ops += 1;
                }
                _ => return Some(left),
            }
        }
    }

    /// The token before is a `)`.
    fn prev_close(&self) -> bool {
        self.at > 0 && self.toks[self.at - 1] == Tok::Close
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.at += 1;
                Some(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op('+')) => {
                self.at += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Option<Expr> {
        let base = self.postfix()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.at += 1;
            let exp = self.unary()?; // right-associative: 2^3^2 = 2^9
            self.binary_ops += 1;
            return Some(Expr::Bin('^', Box::new(base), Box::new(exp)));
        }
        Some(base)
    }

    fn postfix(&mut self) -> Option<Expr> {
        let mut e = self.primary()?;
        while let Some(Tok::Bang) = self.peek() {
            self.at += 1;
            self.calls += 1;
            e = Expr::Fac(Box::new(e));
        }
        Some(e)
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.next()? {
            Tok::Num(v, raw) => {
                self.numbers += 1;
                // decimal comma: 1,5 – only outside function arguments, digits on both sides
                if self.args == 0
                    && self.peek() == Some(&Tok::Comma)
                    && raw.chars().all(|c| c.is_ascii_digit())
                    && let Some(Tok::Num(_, frac)) = self.toks.get(self.at + 1).cloned()
                    && frac.chars().all(|c| c.is_ascii_digit())
                    && let Ok(x) = format!("{raw}.{frac}").parse::<f64>()
                {
                    self.at += 2;
                    return Some(Expr::Num(x));
                }
                Some(Expr::Num(v))
            }
            Tok::Name(n) => {
                if self.peek() == Some(&Tok::Open) {
                    self.at += 1;
                    self.args += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::Close) {
                        loop {
                            args.push(self.expr()?);
                            match self.next()? {
                                Tok::Comma => continue,
                                Tok::Close => break,
                                _ => return None,
                            }
                        }
                    } else {
                        self.at += 1;
                    }
                    self.args -= 1;
                    let ok = (FUNCTIONS.contains(&n.as_str()) && args.len() == 1)
                        || (n == "convert" && args.len() == 3);
                    if !ok {
                        return None;
                    }
                    self.calls += 1;
                    Some(Expr::Call(n, args))
                } else if matches!(n.as_str(), "pi" | "PI" | "Pi" | "e" | "E")
                    || self.vars.contains_key(&n)
                {
                    Some(Expr::Var(n))
                } else {
                    None
                }
            }
            Tok::Open => {
                let e = self.expr()?;
                // a missing ")" at the end is added (template: Arranger)
                match self.next() {
                    Some(Tok::Close) | None => Some(e),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// A parsed expression and whether it is arithmetic worth showing.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub expr: Expr,
    /// At least one operator or function and one number.
    pub arithmetic: bool,
}

/// Parses `text` (variables from `vars`); `None` when it is no expression.
pub fn parse(text: &str, vars: &HashMap<String, f64>) -> Option<Parsed> {
    if text.chars().count() > MAX_LEN {
        return None;
    }
    let toks = lex(text)?;
    if toks.is_empty() {
        return None;
    }
    let mut p = Parser {
        toks,
        at: 0,
        depth: 0,
        args: 0,
        vars,
        binary_ops: 0,
        calls: 0,
        numbers: 0,
    };
    let expr = p.expr()?;
    // a superfluous ")" at the end is ignored (template: Arranger)
    while p.peek() == Some(&Tok::Close) {
        p.at += 1;
    }
    if p.at != p.toks.len() {
        return None;
    }
    Some(Parsed {
        arithmetic: (p.binary_ops > 0 || p.calls > 0) && p.numbers > 0,
        expr,
    })
}

/// n! – exact for whole numbers up to 170 (above: too large for f64), else Γ(n+1).
fn factorial(n: f64) -> f64 {
    if n < 0.0 || n.is_nan() {
        return f64::NAN;
    }
    if n > 170.0 {
        return f64::INFINITY;
    }
    if n.fract() == 0.0 {
        (1..=n as u64).fold(1.0, |acc, k| acc * k as f64)
    } else {
        gamma(n + 1.0)
    }
}

/// Lanczos approximation of Γ.
fn gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

pub fn eval(e: &Expr, vars: &HashMap<String, f64>) -> f64 {
    match e {
        Expr::Num(v) => *v,
        Expr::Var(n) => match n.as_str() {
            "pi" | "PI" | "Pi" => std::f64::consts::PI,
            "e" | "E" => std::f64::consts::E,
            n => vars.get(n).copied().unwrap_or(f64::NAN),
        },
        Expr::Neg(a) => -eval(a, vars),
        Expr::Fac(a) => factorial(eval(a, vars)),
        Expr::Bin(op, a, b) => {
            let (x, y) = (eval(a, vars), eval(b, vars));
            match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => {
                    if y == 0.0 {
                        f64::NAN
                    } else {
                        x / y
                    }
                }
                '%' => x % y,
                '^' => x.powf(y),
                _ => f64::NAN,
            }
        }
        Expr::Call(f, args) => {
            let a = args.iter().map(|x| eval(x, vars)).collect::<Vec<_>>();
            let x = a[0];
            match f.as_str() {
                "sin" => x.sin(),
                "cos" => x.cos(),
                "tan" => x.tan(),
                "asin" => x.asin(),
                "acos" => x.acos(),
                "atan" => x.atan(),
                "sinh" => x.sinh(),
                "cosh" => x.cosh(),
                "tanh" => x.tanh(),
                "radians" | "rad" => x.to_radians(),
                "degrees" => x.to_degrees(),
                "exp" => x.exp(),
                "expm1" => x.exp_m1(),
                "log" | "ln" => x.ln(),
                "log10" => x.log10(),
                "log1p" => x.ln_1p(),
                "sqrt" => x.sqrt(),
                "cbrt" => x.cbrt(),
                "ceil" => x.ceil(),
                "floor" => x.floor(),
                "rint" => {
                    // Java rint: halves to even
                    let r = x.round();
                    if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
                        r - x.signum()
                    } else {
                        r
                    }
                }
                "round" => (x + 0.5).floor(),
                "abs" => x.abs(),
                "ulp" => {
                    let b = x.abs().to_bits();
                    f64::from_bits(b + 1) - x.abs()
                }
                "signum" => {
                    if x == 0.0 || x.is_nan() {
                        x
                    } else {
                        x.signum()
                    }
                }
                "fac" => factorial(x),
                "convert" => {
                    if a[1] == 0.0 {
                        f64::NAN
                    } else {
                        a[0] / a[1] * a[2]
                    }
                }
                _ => f64::NAN,
            }
        }
    }
}

/// The expression written out tidily: `255 / 8`, `2 * sin(pi / 4)`.
pub fn show(e: &Expr) -> String {
    fn prec(e: &Expr) -> u8 {
        match e {
            Expr::Bin('+' | '-', ..) => 1,
            Expr::Bin('*' | '/' | '%', ..) => 2,
            Expr::Neg(_) => 3,
            Expr::Bin('^', ..) => 4,
            _ => 5,
        }
    }
    fn go(e: &Expr, out: &mut String) {
        let wrap = |inner: &Expr, min: u8, out: &mut String| {
            if prec(inner) < min {
                out.push('(');
                go(inner, out);
                out.push(')');
            } else {
                go(inner, out);
            }
        };
        match e {
            Expr::Num(v) => out.push_str(&format_number(*v, None, false)),
            Expr::Var(n) => out.push_str(n),
            Expr::Neg(a) => {
                out.push('-');
                wrap(a, 4, out);
            }
            Expr::Fac(a) => {
                wrap(a, 5, out);
                out.push('!');
            }
            Expr::Bin(op, a, b) => {
                let p = prec(e);
                // left-associative: the right side needs brackets at the same level; ^ the other way
                let (lp, rp) = if *op == '^' { (p + 1, p) } else { (p, p + 1) };
                wrap(a, lp, out);
                out.push_str(&format!(" {op} "));
                wrap(b, rp, out);
            }
            Expr::Call(f, args) => {
                out.push_str(f);
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    go(a, out);
                }
                out.push(')');
            }
        }
    }
    let mut s = String::new();
    go(e, &mut s);
    s
}

/// A result for people: at most 15 significant digits, no `.0`, `decimals` = fixed places,
/// `comma` = decimal comma. `None` for NaN.
pub fn format_number(v: f64, decimals: Option<usize>, comma: bool) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "∞".into() } else { "-∞".into() };
    }
    let s = match decimals {
        Some(d) => format!("{v:.d$}"),
        None => {
            if v == 0.0 {
                "0".into()
            } else if v.abs() >= 1e15 || v.abs() < 1e-6 {
                // scientific, 15 significant digits trimmed
                let s = format!("{v:.14e}");
                let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
                let m = m.trim_end_matches('0').trim_end_matches('.');
                format!("{m}e{e}")
            } else {
                let digits = 15 - (v.abs().log10().floor() as i32 + 1);
                let d = digits.clamp(0, 15) as usize;
                let s = format!("{v:.d$}");
                if s.contains('.') {
                    s.trim_end_matches('0').trim_end_matches('.').to_owned()
                } else {
                    s
                }
            }
        }
    };
    let s = if s == "-0" { "0".to_owned() } else { s };
    if comma { s.replace('.', ",") } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc(s: &str) -> f64 {
        let vars = HashMap::from([("x".to_string(), 3.0)]);
        let p = parse(s, &vars).unwrap_or_else(|| panic!("no expression: {s}"));
        eval(&p.expr, &vars)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    /// The shared cases (`testdata/math.cases.json`): the template's `MathParserTest`, the grammar.
    #[test]
    fn shared_cases() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../testdata/math.cases.json")).unwrap();
        for group in ["template", "grammar"] {
            for c in cases[group].as_array().unwrap() {
                let s = c[0].as_str().unwrap();
                let want = c[1].as_f64().unwrap();
                assert!(close(calc(s), want), "{group} {s}: {} ≠ {want}", calc(s));
            }
        }
        let vars = HashMap::new();
        for s in cases["none"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            let p = parse(s, &vars);
            assert!(p.as_ref().is_none_or(|p| !p.arithmetic), "{s}: {p:?}");
        }
    }

    #[test]
    fn factorial_does_not_overflow_into_nonsense() {
        let v = calc("1/fac(33)");
        assert!(v > 0.0 && close(v, 1.0 / 8.683_317_618_811_886e36), "{v}");
        assert_eq!(calc("fac(171)"), f64::INFINITY);
        assert!(calc("fac(-1)").is_nan());
        assert!(close(calc("fac(0.5)"), 0.886_226_925_452_758), "Γ(1.5)");
    }

    #[test]
    fn limits() {
        let vars = HashMap::new();
        assert!(parse(&"1+".repeat(600), &vars).is_none(), "too long");
        assert!(
            parse(&format!("{}1{}", "(".repeat(100), ")".repeat(100)), &vars).is_none(),
            "too deep"
        );
    }

    #[test]
    fn showing_and_formatting() {
        let p = |s: &str| show(&parse(s, &HashMap::new()).unwrap().expr);
        assert_eq!(p("255/8"), "255 / 8");
        assert_eq!(p("(1+2)*3"), "(1 + 2) * 3");
        assert_eq!(p("1-(2-3)"), "1 - (2 - 3)");
        assert_eq!(p("2^3^2"), "2 ^ 3 ^ 2");
        assert_eq!(p("(2^3)^2"), "(2 ^ 3) ^ 2");
        assert_eq!(p("-2^2"), "-2 ^ 2");
        assert_eq!(p("2 sin(pi/4)"), "2 * sin(pi / 4)");
        assert_eq!(format_number(31.875, None, false), "31.875");
        assert_eq!(format_number(1.0 / 3.0, None, false), "0.333333333333333");
        assert_eq!(format_number(2.0, None, false), "2");
        assert_eq!(format_number(0.1 + 0.2, None, false), "0.3");
        assert_eq!(format_number(1e20, None, false), "1e20");
        assert_eq!(format_number(31.875, Some(2), true), "31,88");
        assert_eq!(format_number(-0.0, None, false), "0");
    }
}

#[cfg(test)]
mod fuzz {
    use super::*;
    use crate::editor::playback::Rng;

    /// Random input never panics and stays fast (deep nesting, long runs, odd characters).
    #[test]
    fn random_input_never_panics() {
        let alphabet: Vec<char> = "0123456789.,+-*/^%()! xepisincofac_×÷−1e9"
            .chars()
            .collect();
        let mut rng = Rng::new(7);
        let vars = HashMap::new();
        let t = web_time::Instant::now();
        for _ in 0..20_000 {
            let n = (rng.next_u64() % 60) as usize;
            let s: String = (0..n)
                .map(|_| alphabet[(rng.next_u64() % alphabet.len() as u64) as usize])
                .collect();
            if let Some(p) = parse(&s, &vars) {
                let _ = eval(&p.expr, &vars);
                let _ = show(&p.expr);
            }
        }
        assert!(t.elapsed().as_secs() < 10);
        let nested = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        assert!(parse(&nested, &vars).is_none());
    }
}
