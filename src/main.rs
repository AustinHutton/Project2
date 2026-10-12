use regex::Regex;
use std::{env, fs, process};

// Each recognized SCW piece receives one of these labels.
#[derive(Debug)]
enum TokenType {
    Keyword,
    Identifier,
    Integer,
    Float,
    Operator,
    Symbol,
    Comment,
}

// Failed candidates receive a broad explanation.
#[derive(Debug)]
enum ErrorCategory {
    InvalidCharacter,
    IllegalNumberFormat,
    InvalidIdentifier,
    InvalidOperator,
}

// Default starts every counter at zero.
#[derive(Default)]
struct TokenCounts {
    keywords: usize,
    identifiers: usize,
    integers: usize,
    floats: usize,
    operators: usize,
    symbols: usize,
    comments: usize,
    errors: usize,
}

// Classify a complete candidate.
fn classify_token(
    text: &str,
    identifier_pattern: &Regex,
    integer_pattern: &Regex,
    float_pattern: &Regex,
    operator_pattern: &Regex,
    symbol_pattern: &Regex,
) -> Option<TokenType> {
    if text.starts_with("//") {
        return Some(TokenType::Comment);
    }
    if float_pattern.is_match(text) {
        return Some(TokenType::Float);
    }
    if integer_pattern.is_match(text) {
        return Some(TokenType::Integer);
    }
    if operator_pattern.is_match(text) {
        return Some(TokenType::Operator);
    }
    if symbol_pattern.is_match(text) {
        return Some(TokenType::Symbol);
    }
    if !identifier_pattern.is_match(text) {
        return None;
    }

    // A keyword also has a valid word shape, so check exact spelling.
    match text {
        "IF" | "ELSE" | "WHILE" | "PRINT" | "INTEGER" | "FLOAT" | "GET" => Some(TokenType::Keyword),
        _ => Some(TokenType::Identifier),
    }
}

fn classify_error(text: &str) -> ErrorCategory {
    // Preserve these unsupported operator sequences as single errors.
    if matches!(text, "=<" | "=>" | "&&" | "|") {
        return ErrorCategory::InvalidOperator;
    }
    if text.starts_with('_') {
        return ErrorCategory::InvalidIdentifier;
    }

    // scan_source() never supplies an empty candidate.
    let first = text.chars().next().expect("Nonempty candidate");
    if first.is_ascii_digit() || (first == '.' && text.len() > 1) {
        return ErrorCategory::IllegalNumberFormat;
    }
    ErrorCategory::InvalidCharacter
}

// Find candidate boundaries, including malformed candidates.
// Keeping "123abc" together lets the classifier report one bad number.
fn scan_source(source: &str, scan_pattern: &Regex) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut remaining = source;

    while !remaining.is_empty() {
        if let Some(found) = scan_pattern.find(remaining) {
            let piece = found.as_str();
            if !piece.trim().is_empty() {
                pieces.push(piece.to_string());
            }
            // Regex offsets are byte positions at character boundaries.
            remaining = &remaining[found.end()..];
        } else {
            // Preserve an unmatched character for error reporting.
            let invalid = remaining.chars().next().expect("Nonempty input");
            pieces.push(invalid.to_string());

            // Advance by its UTF-8 length, avoiding a split inside a character.
            remaining = &remaining[invalid.len_utf8()..];
        }
    }

    pieces
}

fn main() {
    // The input program comes from a file supplied in the terminal.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <source_file.scw>");
        process::exit(1);
    }

    let file_path = &args[1];
    let source = match fs::read_to_string(file_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("Could not read '{}': {}", file_path, error);
            process::exit(1);
        }
    };

    // Compile once, then reuse throughout the file.
    // \A and \z require the WHOLE candidate to match.
    let identifier_pattern =
        Regex::new(r"\A[A-Za-z][A-Za-z0-9_]*\z").expect("Invalid internal identifier regex");
    let integer_pattern = Regex::new(r"\A[0-9]+\z").expect("Invalid internal integer regex");
    let float_pattern = Regex::new(r"\A[0-9]+\.[0-9]+\z").expect("Invalid internal float regex");
    let operator_pattern = Regex::new(r"\A(==|!=|<=|>=|\|\||[+\-*/=<>!&])\z")
        .expect("Invalid internal operator regex");
    let symbol_pattern = Regex::new(r"\A[(){};]\z").expect("Invalid internal symbol regex");

    // This regex consumes only the next prefix of the remaining input.
    // (?x) allows formatting whitespace in the pattern.
    // Comments precede '/', and long operators precede short ones.
    // Broad word/number candidates also collect malformed input for rejection.
    let scan_pattern = Regex::new(
        r"(?x)\A(
            \s+
            | //[^\r\n]*
            | [A-Za-z_][A-Za-z0-9_]*
            | [0-9][A-Za-z0-9_.]*
            | \.[0-9][A-Za-z0-9_.]*
            | =< | => | &&
            | == | != | <= | >= | \|\|
            | [+\-*/=<>!&]
            | [(){};]
        )",
    )
    .expect("Invalid internal scanner regex");

    println!("File: {}\n", file_path);
    let mut counts = TokenCounts::default();

    for text in scan_source(&source, &scan_pattern) {
        match classify_token(
            &text,
            &identifier_pattern,
            &integer_pattern,
            &float_pattern,
            &operator_pattern,
            &symbol_pattern,
        ) {
            Some(kind) => {
                println!("Token: {}, Type: {:?}", text, kind);

                match kind {
                    TokenType::Keyword => counts.keywords += 1,
                    TokenType::Identifier => counts.identifiers += 1,
                    TokenType::Integer => counts.integers += 1,
                    TokenType::Float => counts.floats += 1,
                    TokenType::Operator => counts.operators += 1,
                    TokenType::Symbol => counts.symbols += 1,
                    TokenType::Comment => counts.comments += 1,
                }
            }

            None => {
                counts.errors += 1;
                eprintln!("Lexical error [{:?}]: '{}'", classify_error(&text), text);
            }
        }
    }

    // Sum valid code-token occurrences; comments and errors are separate.
    let total = counts.keywords
        + counts.identifiers
        + counts.integers
        + counts.floats
        + counts.operators
        + counts.symbols;

    println!("\n--- Token count summary ---");
    println!("Total Tokens: {}", total);
    println!("Keywords: {}", counts.keywords);
    println!("Identifiers: {}", counts.identifiers);
    println!("Integers: {}", counts.integers);
    println!("Floats: {}", counts.floats);
    println!("Operators: {}", counts.operators);
    println!("Symbols: {}", counts.symbols);
    println!("Comments (excluded from total): {}", counts.comments);
    println!("Errors: {}", counts.errors);

    // Report all errors first, then signal failure to the terminal.
    if counts.errors > 0 {
        process::exit(1);
    }
}
