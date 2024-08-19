/*
 * @Author: likkoliu
 * @Date: 2024-08-17 10:48:48
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-19 19:54:35
 * @Description:
 */
use serde_derive::Deserialize;
use std::{
    io::{self, Write},
    vec,
};
use thiserror::Error;
use toml;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("data loss")]
    Loss(#[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data error")]
    Unknown,
}

/// Supported command data types
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum GenericCmd {
    Number(u8),
    Character(String),
}

/// Console Status
#[derive(Debug)]
pub enum ConsoleStatus {
    InsAcqFromFile,     // Instruction acquisition status
    InsAcqFromTerminal, // Instruction acquisition status

    InsExecFromFile,     // Instruction execution status
    InsExecFromTerminal, // Instruction execution status

    Invaild, // Invaild state
}

#[derive(Deserialize, Debug)]
pub struct ValidCheck {
    read_valid: bool,   // Command read valid
    import_valid: bool, // Import file is valid
    file_valid: bool,   // The file address has been obtained
}

/// Command line echo prompt
#[derive(Debug)]
pub struct ConsolePrompt {
    mian_prompt: String,
    sub_prompt: String,
}

/// Automation command execution file config
#[derive(Deserialize, Debug)]
pub struct ExcuteFile {
    file_address: Option<String>, // Automatic execution command file address

    exc_ins_assets: Vec<ExcuteAssets>, // Automatically execute instructions and command assets
    cycle_times: Option<u8>,           // Automatic execution cycle times

    next_exc_ins: Option<String>, // Next automatic execution instruction
    next_exc_cmd: Option<String>, // Next auto-execute command
}

#[derive(Deserialize, Debug)]
struct ExcuteAssets {
    exc_ins: Option<GenericCmd>, // Automatic execution instruction
    sub_cmd_assets: Vec<SubCmd>, // Auto-execute command assets
}

#[derive(Deserialize, Debug)]
struct SubCmd {
    sub_cmd: GenericCmd,
}

#[derive(Debug)]
pub struct Console {
    current_status: ConsoleStatus,
    check: ValidCheck,
    interact_prompt: ConsolePrompt,

    current_ins: Option<String>, // Currently executing instruction
    current_cmd: Option<String>, // Currently executing command

    auto_exc: ExcuteFile,
}

impl Console {
    pub fn new() -> Self {
        Console {
            current_status: ConsoleStatus::Invaild,
            check: ValidCheck {
                read_valid: false,
                import_valid: false,
                file_valid: false,
            },
            interact_prompt: ConsolePrompt {
                mian_prompt: String::from("> "),
                sub_prompt: String::from(""),
            },

            current_ins: None,
            current_cmd: None,

            auto_exc: ExcuteFile {
                file_address: None,
                exc_ins_assets: Vec::new(),
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
        self.current_status = ConsoleStatus::Invaild;
    }

    fn input_paser(input: String) -> String {
        let x: &[_] = &['\r', '\n'];
        return String::from(input.trim_end_matches(x));
    }

    pub fn terminal_read(&mut self, prompt: &str) -> Result<GenericCmd, DataError> {
        self.log(&format!(
            "{}{}{}",
            self.interact_prompt.mian_prompt, self.interact_prompt.sub_prompt, prompt
        ));

        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|_| DataError::InvalidHeader {
                expected: ("terminal cmd".to_string()),
                found: ("invalid string".to_string()),
            })?;

        let input = Console::input_paser(input);
        if !input.chars().all(|c| c.is_alphanumeric()) {
            self.check.read_valid = false;
            return Err(DataError::InvalidHeader {
                expected: ("terminal cmd".to_string()),
                found: ("invalid string".to_string()),
            });
        }

        if let ConsoleStatus::InsAcqFromTerminal = self.current_status {
            self.current_ins = Some(input.clone());
        } else {
            self.current_cmd = Some(input.clone());
        }

        self.interact_prompt
            .mian_prompt
            .push_str(&format!("{} > ", input.clone()));
        self.check.read_valid = true;
        
        Ok(GenericCmd::Character(input))
    }

    pub fn file_read(&mut self, prompt: &str) -> Result<GenericCmd, DataError> {
        if let ConsoleStatus::InsAcqFromFile = self.current_status {
        } else {
        }
        Err(DataError::Unknown)
    }

    pub fn file_import(&mut self, prompt: &str) -> Result<(), DataError> {
        self.check.file_valid = true;
        let context = std::fs::read_to_string(prompt)?;

        self.auto_exc = toml::from_str::<ExcuteFile>(&context).map_err(|_| DataError::Unknown)?;

        self.check.import_valid = true;
        println!("{:#?}", self.auto_exc);

        Ok(())
    }

    pub fn read(&mut self, prompt: &str) -> Result<GenericCmd, DataError> {
        let cmd:GenericCmd = match self.current_status {
            ConsoleStatus::InsAcqFromTerminal | ConsoleStatus::InsExecFromTerminal => {
                self.terminal_read(prompt)?
            }
            ConsoleStatus::InsAcqFromFile | ConsoleStatus::InsExecFromFile => {
                self.file_read(prompt)?
            }
            ConsoleStatus::Invaild => { 
                return Err(DataError::Unknown);
            }
        };

        self.refresh();
        Ok(cmd)
    }

    /// Console state machine refresh
    pub fn refresh(&mut self) -> Result<(), DataError> {
        self.current_status = match self.current_status {
            ConsoleStatus::Invaild => {
                self.prompt_clear();
                ConsoleStatus::InsAcqFromTerminal
            }
            ConsoleStatus::InsAcqFromFile => {
                if let Some(_) = self.auto_exc.next_exc_cmd {
                    ConsoleStatus::InsExecFromFile
                } else if let Some(_) = self.auto_exc.next_exc_ins {
                    self.prompt_clear();
                    ConsoleStatus::InsAcqFromFile
                } else {
                    ConsoleStatus::Invaild
                }
            }
            ConsoleStatus::InsAcqFromTerminal => match self.check.read_valid {
                true => {
                    if self.check.file_valid && self.check.import_valid {
                        self.prompt_clear();
                        ConsoleStatus::InsAcqFromFile
                    } else {
                        ConsoleStatus::InsExecFromTerminal
                    }
                }
                false => {
                        ConsoleStatus::Invaild
                    }
                }
            ConsoleStatus::InsExecFromFile => {
                if let Some(_) = self.auto_exc.next_exc_cmd {
                    ConsoleStatus::InsExecFromFile
                } else if let Some(_) = self.auto_exc.next_exc_ins {
                    self.prompt_clear();
                    ConsoleStatus::InsAcqFromFile
                } else {
                    ConsoleStatus::Invaild
                }
            }
            ConsoleStatus::InsExecFromTerminal => {
                if self.check.read_valid {
                    ConsoleStatus::InsExecFromTerminal
                } else {
                    ConsoleStatus::Invaild
                }
            }
        };

        println!(
            "
            + - - - - - - - - - + - - - - - - - - - - - +
            |   控制台当前状态  |  {:?}  
            + - - - - - - - - - + - - - - - - - - - - - +
        ",
            self.current_status
        );

        self.check_reset();
        Ok(())
    }

    /// Clear the console command cache
    pub fn prompt_clear(&mut self) {
        self.interact_prompt.mian_prompt = String::from("> ");
        self.interact_prompt.sub_prompt = String::from("");
    }

    pub fn check_reset(&mut self) {
        self.check.file_valid = false;
        self.check.import_valid = false;
        self.check.read_valid = false;
    }

    pub fn log(&self, log_info: &str) {
        println!("{log_info}");
    }
}
