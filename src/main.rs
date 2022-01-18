use std::io::{self, BufRead, BufReader, Write};
use std::convert::TryFrom;
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
        let dollar = usize::try_from(buffer.len()).unwrap();
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

fn prompt_and_take_input(prompt: &String) -> Result<String, io::Error> {
    let mut input = String::new();

    print!("{}", prompt);
    io::stdout().flush()?;

    match io::stdin().read_line(&mut input) {
        Ok(_) => Ok(String::from(input.trim())),
        Err(error) => Err(error)
    }
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
    let mut comma_first = true;
    let mut multiple_addresses = false;

    while let Some(peek) = input.peek() {
        if peek.is_digit(10) {
            if push_to_addr1 == true {
                addr1.push(*input.pop().unwrap());
            } else {
                addr2.push(*input.pop().unwrap());
                multiple_addresses = true;
            }
            comma_first = false;
            continue;
        } else if *peek == ' ' {
            input.pop();
            continue;
        } else if *peek == ',' {
            push_to_addr1 =
                if push_to_addr1 == true {
                    false
                } else {
                    true
                };
            
            input.pop();
            continue;
        } else {
            break;
        }
    }

    if addr1.is_empty() && !comma_first { return Ok(0) };

    state.address1 = if comma_first {
        state.dot
    } else {
        addr1.parse().unwrap() 
    };

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

    if multiple_addresses {
        state.address2 =  if comma_first {
            state.dollar
        } else {
            addr2.parse().unwrap()
        };

        if state.address1 > state.buffer.len() ||
            state.address2 > state.buffer.len() {
                return Err(AddressError {
                    msg: String::from("some address exceeds eof")
                });
            }

        if state.address1 > state.address2 {
            return Err(AddressError {
                msg: String::from("first address exceeds second address")
            });
        }

        state.dot = state.address2;
        return Ok(2);
    }

    state.dot = state.address1;
    return Ok(1);
}

fn execute_commands(input: &mut EditorInput,
                    state: &mut EditorState,
                    addresses: i32) {

    match input.pop() {
        Some(ichar) => {
            match ichar {
                'p' => {
                    let slice = if addresses > 1 {
                        &state.buffer[state.address1..=state.address2]
                    } else {
                        &state.buffer[state.dot..=state.dot]
                    };
                    
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
                //only the current one if any
                let slice = &state.buffer[state.dot..=state.dot];
                    
                for lines in slice {
                    println!("{}", lines);
                }
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
