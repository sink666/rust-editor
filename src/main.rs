use std::io::{self, BufRead, BufReader, Write};
use std::fs::File;
use std::error::Error;
use argparse::{ArgumentParser,Store};
use tempfile::tempfile;

mod addressing;
use addressing::*;

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

impl Default for EditorConfig {
    fn default() -> Self {
        Self::new()
    }
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

fn command_prompt(prompt: &str) -> Result<String, io::Error> {
    let mut input = String::new();

    print!("{}", prompt);
    io::stdout().flush()?;

    io::stdin()
        .read_line(&mut input)
        .map(|_| String::from(input.trim()))
}

fn execute_commands(input: &mut EditorInput, state: &mut EditorState,
                    num_addrs: i32) {
    match input.pop() {
        Some(ichar) => {
            match ichar {
                'a' => { //enter input mode
                    state.flip_mode();
                },
                'p' => {
                    let slice = &state.buffer[state.address1..=
                                              state.address2];
                    
                    for lines in slice {
                        println!("{}", lines);
                    }
                },
                'Q' => { std::process::exit(0); },
                _ => { println!("?"); },
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut state = EditorState::new(EditorConfig::new());

    loop {
        match &state.current_mode {
            Mode::Command => {
                let temp = command_prompt(&state.prompt)?;
                let mut input = EditorInput::new(&temp);
                let parsed = extract_addresses(&mut input)?;

                match set_addresses(parsed, &mut state) {
                    Ok(num_addrs) => {
                        execute_commands(&mut input, &mut state, num_addrs);
                    },
                    Err(error) => {
                        println!("? : {}", error);
                    },
                }
            },
            Mode::Insert => {
                
            },
        }
    }
}
