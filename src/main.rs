use std::io::{self, BufRead, BufReader, Write};
use std::fs::File;
use argparse::{ArgumentParser,Store};
use tempfile::tempfile;
use std::fmt;

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

#[derive(Debug, Clone)]
pub struct AddressError {
    msg: String
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid address; {}", self.msg)
    }
}

fn extract_addresses(input: &mut EditorInput,
                     state: &mut EditorState) -> Result<i32, AddressError> {
    let mut addr1: String = String::new();
    let mut addr2: String = String::new();
    let mut push_to_addr1 = true;
    let mut comma_first = false;

    while let Some(peek) = input.peek() {
        if peek.is_digit(10) {
            comma_first = false;
            if push_to_addr1 {
                addr1.push(*input.pop().unwrap());
            } else {
                addr2.push(*input.pop().unwrap());
            }
            continue;
        } else if *peek == ' ' {
            input.pop();
            continue;
        } else if *peek == ',' {
            comma_first = true;
            push_to_addr1 = !(push_to_addr1);
            input.pop();
            continue;
        } else {
            break;
        }
    }

    let mut addr_count = 0;

    if addr1.is_empty() && addr2.is_empty() && comma_first {
        state.address1 = 1;
        state.address2 = state.dollar;
        addr_count = 0;
    }

    if addr1.is_empty() && addr2.is_empty() {
        return Ok(addr_count)
    };

    if addr1.is_empty() && !comma_first {
        state.address1 = 1;
        state.address2 = addr2.parse().unwrap();
        addr_count = 1;
    } else if addr2.is_empty() && !comma_first {
        state.address1 = addr1.parse().unwrap();
        state.address2 = addr1.parse().unwrap();
        addr_count = 1;
    } else {
        state.address1 = addr1.parse().unwrap_or(state.dot);
        state.address2 = addr2.parse().unwrap_or(state.dot);
    }

    if state.address1 == 0 {
        return Err(AddressError {
            msg: String::from("cannot address line 0")
        });
    }

    if state.address1 >= state.buffer.len() {
        return Err(AddressError {
            msg: String::from("address exceeds eof")
        });
    }

    if state.address1 > state.address2 {
        return Err(AddressError {
            msg: String::from("first address may not exceed second address")
        });
    }

    if state.address1 > state.buffer.len() ||
        state.address2 > state.buffer.len() {
            return Err(AddressError {
                msg: String::from("some address exceeds eof")
            });
        }

    if addr2.is_empty() { state.address2 = state.address1 }
    state.dot = state.address2;
    
    Ok(addr_count)
}

fn execute_commands(input: &mut EditorInput,
                    state: &mut EditorState,
                    addresses: i32) {

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
            if addresses < 1 {
                println!("?")
            } else {
                //only the current one 
                let slice = &state.buffer[state.dot];
                println!("{}", slice);
            }
        },
    }
}

fn main() {
    let mut state = EditorState::new(EditorConfig::new());

    loop {
        match prompt_and_take_input(&state.prompt) {
            Ok(input) => {
                let mut input = EditorInput::new(&input);
                match extract_addresses(&mut input, &mut state) {
                    Ok(num_addrs) => {
                        execute_commands(&mut input, &mut state, num_addrs);
                    },
                    Err(error) => println!("{}", error),
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
