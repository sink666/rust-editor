use core::fmt;
use std::str;
use std::error::Error;

use crate::EditorState;

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

#[derive(Debug)]
pub enum Value {
    Seperator(char),
    NumericAddr(usize),
    SymbolicAddr(char),
    Empty,
}

impl str::FromStr for Value {
    type Err = AddressError;

    fn from_str(string: &str) -> Result<Self, AddressError> {
        if string.is_empty() {
            Ok(Self::Empty)
        } else if string.chars().all(char::is_numeric) {
            Ok(Self::NumericAddr(string.parse().unwrap()))
        } else if string.chars().all(|x| x == ';' || x == ',') {
            Ok(Self::Seperator(string.parse().unwrap()))
        } else if string.chars().all(|c| ['$', '.', '+', '-'].contains(&c)) {
            Ok(Self::SymbolicAddr(string.parse().unwrap()))
        } else {
            Err(AddressError::WeirdInput(string.to_string()))
        }
    }
}

#[derive(Debug)]
pub enum AddressError {
    LinumError(usize),
    MalformedError,
    WeirdInput(String),
}

impl Error for AddressError {}

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

pub fn extract_addresses(input: &mut EditorInput)
                         -> Result<Vec<Value>, AddressError> {
    let mut addr_buffer: String = String::new();
    let mut addr_vec: Vec<String> = Vec::new();
    let mut split_here = false;
    
    while let Some(peek) = input.peek() {
        if peek.is_digit(10) {
            addr_buffer.push(*input.pop().unwrap());
        } else if ['.','$','+','-',',',';'].contains(peek) {
            split_here = !split_here;
        } else if peek.is_whitespace() {
            input.pop();
        } else if peek.is_alphabetic() {
            if !addr_buffer.is_empty() { addr_vec.push(addr_buffer.clone()) }
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
            split_here = !split_here;
        }

        if input.end_of_line() && !addr_buffer.is_empty() {
            addr_vec.push(addr_buffer.clone());
            addr_buffer.clear();
        }
    }

    if addr_vec.is_empty() { addr_vec.push(String::from("")) }
    
    let parsed = addr_vec
        .into_iter()
        .map(|x| x.parse())
        .collect::<Result<Vec<Value>, AddressError>>()?;

    Ok(parsed)
}

pub fn set_addresses(address_vec: Vec<Value>,
                 state: &mut EditorState) -> Result<i32, AddressError> {
    let mut num_addrs = -1;
    let mut temp_addr1 = state.address1;
    let mut temp_addr2 = state.address2;
    let mut temp_dot = state.dot;
    let mut first = false;
    
    for unit in address_vec {
        match unit {
            Value::NumericAddr(num) => {
                if num_addrs == -1 { num_addrs = 0 }
                temp_addr2 = num;
                num_addrs += 1;
            },
            Value::SymbolicAddr(sym) => {
                if num_addrs == -1 { num_addrs = 0 }
                match sym {
                    '.' => {
                        temp_addr2 = temp_dot;
                        num_addrs += 1;
                    },
                    '$' => {
                        temp_addr2 = state.dollar;
                        num_addrs += 1;
                    },
                    '+' => { temp_addr2 += 1 },
                    '-' => { temp_addr2 -= 1 },
                    _ => {},
                }
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
                            temp_addr2 = state.dollar;
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

    if temp_addr2 > state.dollar || temp_addr2 == 0 {
        return Err(AddressError::LinumError(temp_addr2));
    }

    if temp_addr1 > temp_addr2 {
        return Err(AddressError::MalformedError);
    }

    state.address1 = temp_addr1;
    state.address2 = temp_addr2;
    state.dot = temp_dot;

    // println!("current; a1: {}, a2: {}, dot: {}",
    //           state.address1, state.address2, state.dot);
    Ok(num_addrs)
}

