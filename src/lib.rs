/*
 * @Author: likkoliu
 * @Date: 2024-08-17 10:48:48
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-17 18:48:55
 * @Description:
 */
use std::io::{self, Write};


/// Console Status
pub enum ConsoleStatus {
    InsAcqFromFile,     // Instruction acquisition status
    InsAcqFromTerminal, // Instruction acquisition status

    InsExecFromFile,     // Instruction execution status
    InsExecFromTerminal, // Instruction execution status

    Idle, // Idle state
}

/// Command line echo prompt
pub struct ConsolePrompt {
    mian_prompt: String,
    sub_prompt: String,
}

/// Automation command execution file config
pub struct ExcuteFile<'a, T> {
    file_address: Option<String>, // Automatic execution command file address

    exc_ins_assets: Option<Vec<ExcuteAssets>>, // Automatically execute instructions and command assets
    cycle_times: Option<u8>,                   // Automatic execution cycle times

    next_exc_ins: Option<&'a T>, // Next automatic execution instruction
    next_exc_cmd: Option<&'a T>, // Next auto-execute command
}

struct ExcuteAssets {
    exc_ins: Option<String>,      // Automatic execution instruction
    sub_cmd: Option<Vec<String>>, // Auto-execute command assets
}

struct Console<'a, T> {
    current_status: ConsoleStatus,
    read_valid: bool,
    interact_prompt: ConsolePrompt,

    current_ins: Option<&'a T>, // Currently executing instruction
    current_cmd: Option<&'a T>, // Currently executing command

    auto_exc: ExcuteFile<'a, T>,
}

impl<'a, T> Console<'a, T> {
    pub fn new() -> Self {
        Console {
            current_status: ConsoleStatus::Idle,
            read_valid: false,
            interact_prompt: ConsolePrompt {
                mian_prompt: String::from("> "),
                sub_prompt: String::from(""),
            },

            current_ins: None,
            current_cmd: None,

            auto_exc: ExcuteFile {
                file_address: None,
                exc_ins_assets: None,
                cycle_times: None,
                next_exc_ins: None,
                next_exc_cmd: None,
            },
        }
    }

    pub fn setup(&mut self) {
        self.current_status = ConsoleStatus::InsAcqFromTerminal;
    }

    pub fn taildowm(&mut self) {
        self.current_status = ConsoleStatus::Idle;
    }

    fn ask_excute_file(&mut self) {}

    fn input_paser(input: String) -> String {
        let x: &[_] = &['\r', '\n'];
        return String::from(input.trim_end_matches(x));
    }

    pub fn terminal_read(&mut self, prompt: &str)<T> {
        self.log(&format!("{}{}", self.interact_prompt.mian_prompt, prompt));

        let _ = io::stdout().flush();
        let mut input = String::new();
        if let Ok(_) = io::stdin().read_line(&mut input) {
            input = Console::<&str>::input_paser(input);
        } else {
            input = String::from("");
        }

        if let ConsoleStatus::InsAcqFromTerminal = self.current_status {
            
        } else {

        }
    }

    pub fn auto_exc_read(&mut self, prompt: &str) {
        if let ConsoleStatus::InsAcqFromFile = self.current_status {
            
        } else {

        }
    }

    pub fn read(&mut self, prompt: &str) {
        match self.current_status {
            ConsoleStatus::InsAcqFromTerminal | ConsoleStatus::InsExecFromTerminal => {
                self.terminal_read(prompt);
            }
            ConsoleStatus::InsAcqFromFile | ConsoleStatus::InsExecFromFile => {
                self.auto_exc_read(prompt);
            }
            ConsoleStatus::Idle => {
                return;
            }
        }
    }

    pub fn log(&self, log_info: &str) {
        print!("{log_info}");
    }
}
