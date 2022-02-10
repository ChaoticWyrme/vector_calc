use std::num::ParseFloatError;

use crate::helper::{CalculatorState, Value, Vector};
use once_cell::sync::Lazy;
use pest::prec_climber::{Assoc, Operator, PrecClimber};
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use thiserror::Error;

#[derive(pest_derive::Parser)]
#[grammar = "calc.pest"]
struct CalcParser;

static PREC_CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
    use Assoc::*;
    use Rule::*;

    PrecClimber::new(vec![
        Operator::new(add, Left) | Operator::new(subtract, Left),
        Operator::new(dot, Left) | Operator::new(cross, Left),
        Operator::new(multiply, Left) | Operator::new(divide, Left),
        Operator::new(power, Right)
    ])
});

pub fn parse(input: &str, state: &mut CalculatorState) -> Result<(), ParseError> {
    let pairs = CalcParser::parse(Rule::command, input)?;

    for pair in pairs {
        state.print_debug(3, format!("{:?} : {}", pair.as_rule(), pair.as_str()));
        match pair.as_rule() {
            Rule::variable_assignment => variable_assignment(pair.into_inner(), state)?,
            Rule::ident => {
                let key = pair.as_str();
                match state.get_var(key) {
                    Some(value) => println!("{} = {}", key, value),
                    None => println!("Variable '{}' not found", key),
                }
            }
            Rule::bare_number => println!("{}", parse_value(pair, state)?),
            Rule::expression => println!("{}", parse_expression(pair, state)?),
            Rule::parser_command => parse_parser_command(pair.into_inner(), state),
            _ => unreachable!("Not recognized"),
        }
    }
    Ok(())
}

fn variable_assignment(pairs: Pairs<Rule>, state: &mut CalculatorState) -> Result<(), ParseError> {
    let mut key: Option<String> = None;
    let mut value: Option<Value> = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::ident => key = Some(pair.as_str().to_owned()),
            Rule::value => value = Some(parse_value(pair, state)?),
            Rule::expression => value = Some(parse_expression(pair, state)?),
            _ => state.print_debug(
                2,
                format!("Var Assignment unknown rule: {:?}", pair.as_rule()),
            ),
        }
    }

    state.set_var(
        key.expect("Parsing error in variable name"),
        value.expect("Parsing error in value"),
    );

    Ok(())
}

fn parse_expression(outer_pair: Pair<Rule>, state: &CalculatorState) -> Result<Value, ParseError> {
    let pairs = outer_pair.into_inner();

    PREC_CLIMBER.climb(
        pairs,
        |pair: Pair<Rule>| parse_value(pair, state),
        |lhs: Result<Value, ParseError>, op: Pair<Rule>, rhs: Result<Value, ParseError>| {
            let lhs = lhs?;
            let rhs = rhs?;
            match op.as_rule() {
                Rule::add => lhs + rhs,
                Rule::subtract => lhs - rhs,
                Rule::multiply => lhs * rhs,
                Rule::divide => lhs / rhs,
                Rule::dot => {
                    if lhs.is_vector() && rhs.is_vector() {
                        Ok(lhs.as_vector().dot(&rhs.as_vector()).into())
                    } else {
                        // Err(ParseError::from_pair("Can only do a dot product on two vectors", outer_pair))
                        Err(ParseError::InvalidExpression(
                            "Can only do a dot product on two vectors",
                        ))
                    }
                }
                Rule::cross => {
                    if lhs.is_vector() && rhs.is_vector() {
                        lhs.as_vector()
                            .cross(&rhs.as_vector())
                            .map(Value::Vector)
                    } else {
                        Err(ParseError::InvalidExpression(
                            "Can only do a cross product on two vectors",
                        ))
                    }
                }
                _ => unreachable!("parse_expression unknown operator rule"),
            }
        },
    )
}

fn parse_parser_command(mut pairs: Pairs<Rule>, state: &mut CalculatorState) {
    let command_type = pairs.next().unwrap();

    match command_type.as_rule() {
        Rule::parser_debug => {
            let data = pairs.next();
            if let Some(debug_level_pair) = data {
                let debug_level: u32 = debug_level_pair
                    .as_str()
                    .parse()
                    .expect("Grammar only allows a single number, so this should never happen");
                state.debug_level = debug_level;
                state.print_debug(1, format!("Changed debug level to {}", debug_level));
            } else {
                println!("Debug level: {}", state.debug_level);
            }
        }
        Rule::parser_modify => modify_variable(
            pairs
                .next()
                .expect("Grammar expects an identifier here")
                .as_str(),
            state,
        ),
        Rule::parser_exit => {
            std::process::exit(0);
        },
        Rule::parser_save => save_state(pairs.next().expect("Grammar expects something here").as_str(), state),
        Rule::parser_load => load_state(pairs.next().expect("Grammar expects something here").as_str(), state),
        _ => unreachable!("Unknown parser command"),
    }
}

const STATE_FILE_EXT: &str = "vecalc";

pub fn save_state(filename: &str, state: &CalculatorState) {
    let mut data = String::new();

    for (name, val) in state.variables.iter() {
        data.push_str(&format!("{} = {}\n", name, val))
    }

    data.push_str(&format!(".debug {}", state.debug_level));

    let err = std::fs::write(&format!("{}.{}", filename, STATE_FILE_EXT), data);
    if let Err(err) = err {
        eprintln!("Error write state file: {}", err);
    }
}

pub fn load_state(filename: &str, state: &mut CalculatorState) {
    use std::io::prelude::*;

    state.debug_level = 0;
    match std::fs::File::open(&format!("{}.{}", filename, STATE_FILE_EXT)) {
        Ok(file) => { 
            let mut reader = std::io::BufReader::new(file);
            let mut line: String = String::new();

            let mut num_lines: usize = 0;
    
            loop {
                let len = reader.read_line(&mut line).expect("reading from the cursor won't fail");
                if len == 0 {
                    break;
                }
                parse(&line, state).unwrap();
                num_lines += 1;
            }

            println!("Processed {} lines", num_lines);
        },
        Err(err) => eprintln!("Error opening state file: {}", err),
    }

    println!("Finished loading state file.")
}

fn modify_variable(var_name: &str, state: &mut CalculatorState) {
    if !state.contains_key(var_name) {
        println!("Unknown variable {}", var_name);
        return;
    }

    let data_enum = state
        .get_var(var_name)
        .expect("Already checked existence")
        .clone();

    let mut rl = rustyline::Editor::<()>::new();

    let prompt = format!("Change {var_name} from {data_enum} to: ");
    let result = rl.readline_with_initial(&prompt, data_enum.to_string().split_at(1));

    if let Ok(str_result) = result {
        let parser_result = CalcParser::parse(Rule::value, &str_result);
        if let Ok(mut value_pairs) = parser_result {
            let value = parse_value(
                value_pairs.next().expect("Grammar specifies existence"),
                state,
            )
            .unwrap();
            let change_result = state.change_var(var_name.to_owned(), value);
            if change_result {
                println!("Changed {var_name}")
            } else {
                println!("Failed to change {var_name} because of differing value")
            }
        } else {
            println!("Failed to parse value");
        }
    } else {
        println!("Rustyline error");
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Tokenization error: {0}")]
    PestError(#[from] pest::error::Error<Rule>),
    #[error("Float parsing error: {0}")]
    ValueParseError(#[from] ParseFloatError),
    #[error("Invalid identifier '{token}'")]
    InvalidIdentifier {
        token: String,
        // position: u32
    },
    #[error("Invalid operator '{token}'")]
    InvalidOperator {
        token: String,
        // position: u32
    },
    // TODO: Add separate type for TypeError for use in problems in operators / functions
    // TODO: Add slot for position of expression, since we have that information
    #[error("Invalid expression: {0}")]
    InvalidExpression(&'static str),

    #[error("Invalid expression: {msg} in the expression from {start} to {end}")]
    InvalidExpr {
        msg: &'static str,
        start: usize,
        end: usize,
    },
}

impl ParseError {
    pub fn from_pair(msg: &'static str, pair: Pair<Rule>) -> Self {
        let span = pair.as_span();
        Self::InvalidExpr {
            msg,
            start: span.start(),
            end: span.end(),
        }
    }
}

fn parse_value(pair: Pair<Rule>, state: &CalculatorState) -> Result<Value, ParseError> {
    state.print_debug(3, format!("(parse_value) rule: {:?}", pair.as_rule()));
    state.print_debug(3, format!("(parse_value) data: '{}'", pair.as_str()));
    match pair.as_rule() {
        Rule::bare_number => Ok(Value::Number(pair.as_str().parse::<f32>()?)),
        Rule::vector => Ok(Value::Vector(parse_vector(pair.into_inner())?)),
        Rule::ident => {
            if let Some(value) = state.get_var(pair.as_str()) {
                Ok(value.to_owned())
            } else {
                return Err(ParseError::InvalidIdentifier {
                    token: pair.as_str().to_string(),
                });
            }
        }
        _ => unreachable!("non-value being parsed as value"),
    }
}

fn parse_vector(pairs: Pairs<Rule>) -> Result<Vector, ParseFloatError> {
    let mut values: Vec<f32> = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::bare_number => values.push(pair.as_str().parse()?),
            _ => unreachable!("Non-number inside of vec"),
        }
    }

    Ok(values.into())
}
