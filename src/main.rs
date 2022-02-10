//extern crate pest;
//#[macro_use]
//extern crate pest_derive;

use rustyline::error::ReadlineError;
use rustyline::Editor;

pub mod helper;
pub mod parser;

use helper::CalculatorState;

fn main() {
    // <()> means no completer
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history");
    }

    let mut state = CalculatorState::new();
    
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let result = parser::parse(line.as_str(), &mut state);
                if let Err(err) = result {
                    eprintln!("ERR: {}", err);
                } else {
                    rl.add_history_entry(line.as_str());
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
            
        }
    }
    rl.save_history("history.txt").unwrap();
}