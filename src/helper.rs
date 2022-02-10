use std::collections::HashMap;
use std::ops::{self, Add, Sub, Mul, Div};

use rustyline::{Helper, validate::Validator, highlight::Highlighter, hint::Hinter, completion::Completer};

use crate::parser::ParseError;

#[derive(Debug, PartialEq, Clone)]
pub struct Vector(Vec<f32>);

impl ops::Deref for Vector {
    type Target = Vec<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<f32>> for Vector {
    fn from(source: Vec<f32>) -> Self {
        Self(source)
    }
}

impl FromIterator<f32> for Vector {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        iter.into_iter().collect::<Vec<f32>>().into()
    }
}

impl Add<Vector> for Vector {
    type Output = Vector;

    fn add(self, rhs: Vector) -> Self::Output {
        self.iter().zip(rhs.iter()).map(|(x, y)| x + y).collect()
    }
}

impl Sub<Vector> for Vector {
    type Output = Vector;

    fn sub(self, rhs: Vector) -> Self::Output {
        self.iter().zip(rhs.iter()).map(|(x, y)| x - y).collect()
    }
}

impl Mul<f32> for Vector {
    type Output = Vector;

    fn mul(self, rhs: f32) -> Self::Output {
        self.iter().map(|x| x * rhs).collect()
    }
}

impl Mul<Vector> for f32 {
    type Output = Vector;

    fn mul(self, rhs: Vector) -> Self::Output {
        rhs * self
    }
}

impl Div<f32> for Vector {
    type Output = Vector;

    fn div(self, rhs: f32) -> Self::Output {
        self.iter().map(|x| x / rhs).collect()
    }
}

impl Vector {
    pub fn length(&self) -> f32 {
        self.mag()
    }

    pub fn mag(&self) -> f32 {
        self.iter().map(|&x| x.powi(2)).sum::<f32>().sqrt()
    }
    
    pub fn dims(&self) -> usize {
        self.0.len()
    }

    pub fn dot(&self, rhs: &Vector) -> f32 {
        self.iter().zip(rhs.iter()).map(|(&x, &y)| x * y).sum()
    }

    pub fn cross(&self, rhs: &Vector) -> Result<Vector, ParseError> {
        if self.dims() != 3 && rhs.dims() != 3 {
            return Err(ParseError::InvalidExpression("Cross product is only between two vectors, both in 3 dimensions"))
        }

        Ok(Vector(vec![
            // c_x = a_y * b_z − a_z * b_y
            (self[1] * rhs[2]) - (self[2] * rhs[1]),
            // c_y = a_z * b_x − a_x * b_z
            (self[2] * rhs[0]) - (self[0] * rhs[2]),
            // c_z = a_x * b_y − a_y * b_x	
            (self[0] * rhs[1]) - (self[1] * rhs[0])
        ]))
    }

    pub fn angle_between(&self, other: &Vector) -> f32 {
        (self.dot(other) / (self.mag() * other.mag())).acos()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Number(f32),
    Vector(Vector),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Print using normal rust formatting for floats
            Value::Number(val) => f.write_fmt(format_args!("{}", val)),
            // print as <1.0, 2.0>
            // or as <Empty Vector> if empty
            Value::Vector(vec) => {
                let mut iter = vec.iter();
                let mut next = iter.next();
                match next {
                    Some(val) => {
                        f.write_str("<")?;
                        f.write_fmt(format_args!("{}", val))?;
                    },
                    None => {
                        return f.write_str("<Empty Vector>");
                    }
                }
                loop {
                    next = iter.next();
                    match next {
                        Some(val) => {
                            f.write_fmt(format_args!(", {}", val))?;
                        },
                        None => {
                            return f.write_str(">");
                        }
                    }
                }
            }
        }
    }
}

impl From<f32> for Value {
    fn from(source: f32) -> Self {
        Self::Number(source)
    }
}

impl From<Vector> for Value {
    fn from(source: Vector) -> Self {
        Self::Vector(source)
    }
}

impl From<Vec<f32>> for Value {
    fn from(source: Vec<f32>) -> Self {
        source.into()
    }
}

impl Value {
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn is_vector(&self) -> bool {
        matches!(self, Value::Vector(_))
    }

    pub fn compare_types(&self, other: &Value) -> bool {
        (self.is_number() && other.is_number()) ||
        (self.is_vector() && other.is_vector())
    }

    /// Panics if the value is not a number
    pub fn as_number(&self) -> f32 {
        match self {
            Self::Number(val) => *val,
            Self::Vector(_) => panic!("Tried to get a number from a vector value")
        }
    }

    pub fn as_vector(&self) -> Vector {
        match self {
            Self::Vector(val) => val.clone(),
            Self::Number(_) => panic!("Tried to get a vector from a number value")
        }
    }
}

impl Add<Value> for Value {
    type Output = Result<Value, ParseError>;

    fn add(self, rhs: Value) -> Self::Output {
        if self.compare_types(&rhs) {
            if self.is_number() {
                Ok(Value::Number(self.as_number() + rhs.as_number()))
            } else if self.is_vector() {
                Ok(Value::Vector(self.as_vector() + rhs.as_vector()))
            } else {
                unreachable!("No other types");
            }
        } else {
            Err(ParseError::InvalidExpression("Can't add a scalar and a vector together"))
        }
    }
}

impl Sub for Value {
    type Output = Result<Value, ParseError>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.compare_types(&rhs) {
            if self.is_number() {
                Ok(Value::Number(self.as_number() - rhs.as_number()))
            } else if self.is_vector() {
                Ok(Value::Vector(self.as_vector() - rhs.as_vector()))
            } else {
                unreachable!("No other types");
            }
        } else {
            Err(ParseError::InvalidExpression("Can't subtract a scalar and a vector"))
        }
    }
}

impl Mul for Value {
    type Output = Result<Value, ParseError>;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_vector() && rhs.is_number() {
            Ok(Value::Vector(self.as_vector() * rhs.as_number()))
        } else if self.is_number() && rhs.is_vector() {
            Ok(Value::Vector(self.as_number() * rhs.as_vector()))
        } else if self.is_number() && rhs.is_number() {
            Ok(Value::Number(self.as_number() * rhs.as_number()))
        } else if self.is_vector() && rhs.is_vector() {
            Err(ParseError::InvalidExpression("Can't multiply two vectors"))
        } else {
            unreachable!("Compared all possible types")
        }
    }
}

impl Div for Value {
    type Output = Result<Value, ParseError>;

    fn div(self, rhs: Self) -> Self::Output {
        if self.is_number() && rhs.is_number() {
            Ok(Value::Number(self.as_number() / rhs.as_number()))
        } else if self.is_vector() && rhs.is_number() {
            Ok(Value::Vector(self.as_vector() / rhs.as_number()))
        } else if self.is_number() && rhs.is_vector() {
            Err(ParseError::InvalidExpression("Can't divide a scalar by a vector"))
        } else if self.is_vector() && rhs.is_vector() {
            Err(ParseError::InvalidExpression("Can't divide a vector by a vector"))
        } else {
            unreachable!("Compared all possible types")
        }
    }
}

pub struct CalculatorState {
    pub(crate) variables: HashMap<String, Value>,
    pub debug_level: u32,
}

const DEFAULT_DEBUG_LEVEL: u32 = 1;

impl Default for CalculatorState {
    fn default() -> Self {
        Self {
            variables: Default::default(),
            debug_level: DEFAULT_DEBUG_LEVEL,
        }
    }
}

impl CalculatorState {
    pub fn new() -> Self { 
        Self {
            variables: HashMap::new(),
            debug_level: DEFAULT_DEBUG_LEVEL
        }
     }

    pub fn new_with_variables(variables: HashMap<String, Value>) -> Self {
        Self {
            variables,
            debug_level: DEFAULT_DEBUG_LEVEL
        }
    }

    pub fn set_var(&mut self, key: String, value: Value) -> Option<Value> {
        self.variables.insert(key, value)
            //.map_or(false, |old_val| old_val != value)
    }

    pub fn change_var(&mut self, key: String, value: Value) -> bool {
        let old_val = self.get_var(&key);
        if old_val.is_none() {
            return false;
        }
        let old_val = old_val.unwrap();

        if value.compare_types(old_val) {
            self.set_var(key, value);
            true
        }  else {
            false
        }
    }

    pub fn get_var(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.variables.contains_key(key)
    }

    pub fn print_debug(&self, min_debug_level: u32, string: String) {
        if self.debug_level >= min_debug_level {
            CalculatorState::debug_print(min_debug_level, string);
        }
    }

    fn debug_print(debug_level: u32, string: String) {
        // TODO: Print different format strings for different debug levels.
        println!("Debug {}: {}", debug_level, string);
    }
}

impl Helper for CalculatorState {}

impl Validator for CalculatorState {
    fn validate(&self, ctx: &mut rustyline::validate::ValidationContext) -> rustyline::Result<rustyline::validate::ValidationResult> {
        let _ = ctx;
        Ok(rustyline::validate::ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

impl Highlighter for CalculatorState {

}

impl Hinter for CalculatorState {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        let _ = (line, pos, ctx);
        None
    }
}

impl Completer for CalculatorState {
    type Candidate = String;

    fn complete(
        &self, // FIXME should be `&mut self`
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let _ = (line, pos, ctx);
        Ok((0, Vec::with_capacity(0)))
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}