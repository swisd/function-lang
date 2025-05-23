mod differential;
mod dmath;

use std::collections::HashMap;
use std::io::{self, Write};
use std::fs;

use pest::Parser;
use pest_derive::Parser;
// use crate::Stmt::Expr;

struct State {
    vars: HashMap<String, f64>,
    funcs: HashMap<String, (String, Expr)>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    FuncDef(String, String, Expr),
    Assign(String, Expr),
    Print(Expr),
    Expr(Expr),
}


#[derive(Parser)]
#[grammar = "math.pest"]
struct MathParser;

#[derive(Debug, Clone)]
enum Expr {
    Number(f64),
    Variable(String),
    UnaryOp { op: String, expr: Box<Expr> },
    BinaryOp { left: Box<Expr>, op: String, right: Box<Expr> },
    FunctionCall { name: String, args: Vec<Expr> },
    Assignment { name: String, value: Box<Expr> },
    FunctionDef { name: String, param: String, body: Box<Expr> },
    Print(Box<Expr>),
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::number => Expr::Number(pair.as_str().parse().expect("...")),
        Rule::ident => Expr::Variable(pair.as_str().to_string()),
        Rule::function_call => {
            let mut inner = pair.into_inner();
            let name = inner.next().expect("...").as_str().to_string();
            let args = inner.map(parse_expr).collect();
            Expr::FunctionCall { name, args }
        }
        Rule::unary => {
            let mut inner = pair.into_inner();
            let first = inner.next().expect("...");
            if first.as_rule() == Rule::primary {
                parse_expr(first)
            } else {
                let op = first.as_str().to_string();
                let expr = parse_expr(inner.next().expect("..."));
                Expr::UnaryOp { op, expr: Box::new(expr) }
            }
        }
        Rule::power | Rule::product | Rule::sum => {
            println!("{}", pair);
            let mut inner = pair.into_inner();
            let mut expr = parse_expr(inner.next().expect("..."));
            while let Some(op) = inner.next() {
                let right = parse_expr(inner.next().expect("Expected right-hand expression"));
                expr = Expr::BinaryOp {
                    left: Box::new(expr),
                    op: op.as_str().to_string(),
                    right: Box::new(right),
                };
            }
            expr
        }
        Rule::assignment => {
            let mut inner = pair.into_inner();
            let name = inner.next().expect("...").as_str().to_string();
            let value = parse_expr(inner.next().expect("..."));
            Expr::Assignment { name, value: Box::new(value) }
        }
        Rule::function_def => {
            let mut inner = pair.into_inner();
            let name = inner.next().expect("Expected function name").as_str().to_string();
            let param = inner.next().expect("Expected function parameter").as_str().to_string();
            let body = parse_expr(inner.next().expect("Expected function body"));
            Ok(Expr::FunctionDef(name, param, body))
        }
        Rule::print_stmt => {
            let inner = pair.into_inner().next().expect("...");
            Expr::Print(Box::new(parse_expr(inner)))
        }
        Rule::expression | Rule::statement => parse_expr(pair.into_inner().next().expect("...")),
        Rule::primary => parse_expr(pair.into_inner().next().expect("...")),
        _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
    }
}

fn eval(expr: Expr, state: &mut State) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(n),
        Expr::Variable(name) => match name.as_str() {
            "pi" => Ok(std::f64::consts::PI),
            "e" => Ok(std::f64::consts::E),
            _ => state.vars.get(&name).copied().ok_or_else(|| format!("Undefined variable: {}", name)),
        },
        Expr::UnaryOp { op, expr } => {
            let val = eval(*expr, state)?;
            match op.as_str() {
                "+" => Ok(val),
                "-" => Ok(-val),
                _ => Err(format!("Unknown unary operator: {}", op)),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = eval(*left, state)?;
            let r = eval(*right, state)?;
            match op.as_str() {
                "+" => Ok(l + r),
                "-" => Ok(l - r),
                "*" => Ok(l * r),
                "/" => Ok(l / r),
                "^" => Ok(l.powf(r)),
                _ => Err(format!("Unknown operator: {}", op)),
            }
        }
        Expr::Assignment { name, value } => {
            let val = eval(*value, state)?;   // FIXED here
            state.vars.insert(name, val);
            Ok(val)
        }
        Expr::FunctionCall { name, args } => {
            if let Some(func) = state.funcs.get(&name).cloned() { // clone tuple to avoid borrow
                let (param, body) = func;
                let arg_val = eval(args[0].clone(), state)?;  // safe now
                if args.len() != 1 {
                    return Err(format!("Function '{}' expects 1 argument", name));
                }
                let mut local_vars = state.vars.clone(); // copy
                local_vars.insert(param.clone(), arg_val);
                let mut inner_state = State {
                    vars: local_vars,
                    funcs: state.funcs.clone(), // keep global funcs
                };
                eval(body.clone(), &mut inner_state)
            } else {
                // fallback to built-in
                let values: Result<Vec<f64>, _> = args.into_iter().map(|a| eval(a, state)).collect();
                let values = values?;
                match (name.as_str(), values.as_slice()) {
                    ("sin", [x]) => Ok(x.sin()),
                    ("cos", [x]) => Ok(x.cos()),
                    ("max", [a, b]) => Ok(f64::max(*a, *b)),
                    _ => Err(format!("Unknown function: {}", name)),
                }
            }
        }
        Expr::FunctionDef { name, param, body } => {
            state.funcs.insert(name, (param, *body));
            Ok(0.0) // or just acknowledge
        }
        Expr::Print(expr) => {
            let value = eval(*expr, state)?;
            println!("{}", value);
            Ok(value)
        }
    }
}

fn run_file(filename: &str, state: &mut State) {
    match fs::read_to_string(filename) {
        Ok(contents) => {
            for (i, line) in contents.lines().enumerate() {
                if line.trim().is_empty() { continue; }
                let parse_result = MathParser::parse(Rule::statement, line);
                match parse_result {
                    Ok(mut pairs) => {
                        let expr = parse_expr(pairs.next().expect("..."));
                        match eval(expr, state) {
                            Ok(result) => println!("Line {}: {} = {}", i + 1, line, result),
                            Err(e) => println!("Line {}: Error evaluating '{}': {}", i + 1, line, e),
                        }
                    }
                    Err(e) => println!("Line {}: Parse error: {}", i + 1, e),
                }
            }
        }
        Err(e) => println!("Could not read file: {}", e),
    }
}

fn main() {
    let mut state = State {
        vars: HashMap::new(),
        funcs: HashMap::new(),
    };

    loop {
        print!("> ");
        io::stdout().flush().expect("...");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input.");
            continue;
        }

        if input.trim() == "exit" {
            break;
        }

        let parse_result = MathParser::parse(Rule::statement, &input);
        match parse_result {
            Ok(mut pairs) => {
                let expr = parse_expr(pairs.next().expect("..."));
                match eval(expr, &mut state) {
                    Ok(result) => println!("= {}", result),
                    Err(e) => println!("Error: {}", e),
                }
            }
            Err(e) => println!("Parse error: {}", e),
        }
    }
}