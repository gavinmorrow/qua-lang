#![expect(dead_code, reason = "prototyping")]

use std::io;

mod ast;
mod lexer;
mod parser;
mod stream;

fn main() {
    // start repl
    eprintln!("glang v0.1.0");
    eprint!("> ");
    while let Some(Ok(line)) = io::stdin().lines().next() {
        run(line);
        eprint!("> ");
    }
    eprintln!("Goodbye! o/");
}

fn run(source: String) {
    let tokens = lexer::lex(source);
    println!("{tokens:#?}");
}
