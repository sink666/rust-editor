use core::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::fs::File;
use argparse::{ArgumentParser,Store};
use tempfile::tempfile;
use std::str;

#[derive(Debug)]
pub enum Mode {
    Command,
    Insert,
}

pub struct EditorState {
    prompt: String,
    current_mode: Mode,
    current_file: File,
    buffer: Vec<String>,
    dot: usize,
    dollar: usize,
    address1: usize,
    address2: usize,
}

impl EditorState {
    pub fn new(config: EditorConfig) -> Self{
        let mut buffer = Self::file_to_vec(&config.openfile);
        buffer.insert(0, "".to_string());
        let dollar = buffer.len() - 1;
        Self {
            prompt: config.prompt,
            current_mode: Mode::Command,
            current_file: config.openfile,
            buffer,
            dot: dollar,
            dollar,
            address1: dollar,
            address2: dollar,
        }
    }

    pub fn flip_mode(&mut self) {
        match self.current_mode {
            Mode::Command => {
                self.current_mode = Mode::Insert;
            }
            Mode::Insert => {
                self.current_mode = Mode::Command;
            }
        }
    }

    fn file_to_vec(file: &File) -> Vec<String> {
        let buf = BufReader::new(file);

        buf.lines()
            .map(|l| l.expect("could not parse line"))
            .collect()
    }        
}

pub struct EditorConfig {
    prompt: String,
    openfile: File,
}

impl EditorConfig {
    pub fn new() -> Self {
        let mut prompt = String::from("");
        let mut filename = String::from("");
        
        { //begin arg parse
            let mut ap = ArgumentParser::new();
            
            ap.set_description("Edit text files.");
            ap.refer(&mut prompt)
                .add_option(&["-p"], Store, "Set a prompt string.");
            ap.refer(&mut filename)
                .add_argument("File", Store, "File to operate on.");
            ap.parse_args_or_exit();
        }
        
        let openfile: File = if filename.is_empty() {
            tempfile().unwrap()
        } else {
            File::open(filename).expect("Could not open file")
        };
        
        Self {
            prompt,
            openfile
        }
    }
}

fn prompt_and_take_input(prompt: &str) -> Result<String, io::Error> {
    let mut input = String::new();

    print!("{}", prompt);
    io::stdout().flush()?;

    io::stdin()
        .read_line(&mut input)
        .map(|_| String::from(input.trim()))
}

#[derive(Debug)]
pub struct EditorInput {
    point: usize,
    line: Vec<char>,
}

impl EditorInput {
    pub fn new(string: &str) -> Self {
        Self {
            point: 0,
            line: string.chars().collect(),
        }
    }

    pub fn point(&self) -> usize {
        self.point
    }

    pub fn peek(&self) -> Option<&char> {
        self.line.get(self.point)
    }

    pub fn end_of_line(&self) -> bool {
        self.point == self.line.len()
    }

    pub fn pop(&mut self) -> Option<&char> {
        match self.line.get(self.point) {
            Some(character) => {
                self.point += 1;
                Some(character)
            }
            None => None,
        }
    }
}

fn extract_addresses(input: &mut EditorInput) -> Vec<String> {
    let mut addr_buffer: String = String::new();
    let mut addr_vec: Vec<String> = Vec::new();
    let mut split_here = false;
    
    while let Some(peek) = input.peek() {
        if peek.is_digit(10) {
            addr_buffer.push(*input.pop().unwrap());
        } else if *peek == ',' || *peek == ';' {
            split_here = !(split_here);
        } else if *peek == ' ' {
            input.pop();
        } else if peek.is_alphabetic() {
            addr_vec.push(addr_buffer.clone());
            break;
        } else {
            break;
        }

        if split_here {
            if !addr_buffer.is_empty() {
                addr_vec.push(addr_buffer.clone());
                addr_buffer.clear();
            }
            addr_buffer.push(*input.pop().unwrap());
            addr_vec.push(addr_buffer.clone());
            addr_buffer.clear();
            split_here = !(split_here);
        }

        if input.end_of_line() && !addr_buffer.is_empty() {
            addr_vec.push(addr_buffer.clone());
            addr_buffer.clear();
        }
    }
    
    addr_vec
}

#[derive(Debug)]
enum Value {
    Seperator(char),
    NumericAddr(usize),
    Empty,
}

impl str::FromStr for Value {
    type Err = AddressError;

    fn from_str(string: &str) -> Result<Self, AddressError> {
        if string.is_empty() {
            Ok(Value::Empty)
        } else if string.chars().all(char::is_numeric) {
            Ok(Value::NumericAddr(string.parse().unwrap()))
        } else if string.chars().all(|x| x == ';' || x == ',') {
            Ok(Value::Seperator(string.parse().unwrap()))
        } else {
            Err(AddressError::WeirdInput(string.to_string()))
        }
    }
}

#[derive(Debug)]
enum AddressError {
    LinumError(usize),
    MalformedError,
    WeirdInput(String),
}

impl std::error::Error for AddressError {}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::LinumError(num) => {
                write!(f, "{}: Invalid Line number", num) },
            Self::MalformedError => {
                write!(f, "Address malformed")
            },
            Self::WeirdInput(string) => {
                write!(f, "{}: unsupported address num", string)
            },
        }
    }
}

fn set_addresses(address_vec: Vec<String>,
                 state: &mut EditorState) -> Result<i32, AddressError> {
    let mut num_addrs = -1;
    let mut temp_addr1 = state.address1;
    let mut temp_addr2 = state.address2;
    let mut temp_dot;
    let mut first = false;
    
    let parsed = address_vec
        .into_iter()
        .map(|x| x.parse())
        .collect::<Result<Vec<Value>, AddressError>>()?;

    for unit in parsed {
        match unit {
            Value::NumericAddr(num) => {
                if num_addrs == -1 { num_addrs = 0 }
                temp_addr2 = num;
                num_addrs += 1;
            },
            Value::Seperator(c) => {
                match c {
                    ',' => {
                        temp_addr1 = temp_addr2;

                        if num_addrs <= 0 {
                            num_addrs = 0;
                            temp_addr1 = 1;
                            temp_addr2 = state.dollar;
                            first = !(first);
                        }
                    },
                    ';' => {
                        temp_dot = temp_addr2;
                        temp_addr1 = temp_addr2;

                        if num_addrs <= 0 {
                            num_addrs = 0;
                            temp_addr1 = temp_dot;
                            first = !first;
                        }
                    },
                    _ => {},
                }
            },
            Value::Empty => {
                temp_addr1 = state.address1;
                temp_addr2 = state.address2;
            },
        }
    }

    if num_addrs <= 1 && !first {
        temp_addr1 = temp_addr2;
    }
    temp_dot = temp_addr2;

    if temp_addr1 > state.dollar || temp_addr1 == 0 {
        return Err(AddressError::LinumError(temp_addr1));
    }

    if temp_addr1 > state.dollar || temp_addr2 == 0 {
        return Err(AddressError::LinumError(temp_addr2));
    }

    if temp_addr1 > temp_addr2 {
        return Err(AddressError::MalformedError);
    }

    state.address1 = temp_addr1;
    state.address2 = temp_addr2;
    state.dot = temp_dot;

    println!("current; a1: {}, a2: {}, dot: {}",
             state.address1, state.address2, state.dot);
    Ok(num_addrs)
}

fn execute_commands(input: &mut EditorInput,
                    state: &mut EditorState,
                    num_addrs: i32) {

    match input.pop() {
        Some(ichar) => {
            match ichar {
                'p' => {
                    let slice = &state.buffer[state.address1..=
                                              state.address2];
                    
                    for lines in slice {
                        println!("{}", lines);
                    }
                },
                'Q' => { std::process::exit(0); },
                _ => { println!("?") },
            }
        },
        None => {
            if num_addrs >= 0 {
                let slice = &state.buffer[state.dot];
                println!("{}", slice);
            } else {
                println!("?");
            }
        },
    }
}

fn main() -> Result<(), io::Error> {
    let mut state = EditorState::new(EditorConfig::new());

    loop {
        let input = prompt_and_take_input(&state.prompt)?;
        let mut input = EditorInput::new(&input);
        
        match set_addresses(extract_addresses(&mut input), &mut state) {
            Ok(num_addrs) => {
                execute_commands(&mut input, &mut state, num_addrs);
            },
            Err(error) => {
                println!("? : {}", error);
            },
        }
    }
}
