/*
 * @Author: likkoliu
 * @Date: 2024-08-17 10:48:48
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-20 17:13:02
 * @Description:
 */
use serde_derive::Deserialize;
use std::fmt;
use std::io::{self, Write};
use thiserror::Error;
use toml;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("data loss")]
    Loss(#[from] io::Error),
    #[error("{0}")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data error")]
    Unknown,
}

/// Supported command data types
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum GenericCmd {
    Number(usize),
    Character(String),
}

/// Console Status
#[derive(Debug, Clone, PartialEq)]
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
    cycle_times: Option<usize>,        // Automatic execution cycle times

    next_exc_ins: Option<(usize, GenericCmd)>, // Next automatic execution instruction
    next_exc_cmd: Option<(usize, GenericCmd)>, // Next auto-execute command
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
    previous_status: ConsoleStatus,
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
            previous_status: ConsoleStatus::Invaild,
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
        let _ = self.refresh();
    }

    pub fn taildowm(&mut self) {
        let _ = self.refresh();
    }

    /// Terminal input character parser
    fn input_parser(&self, input: String) -> String {
        let x: &[_] = &['\r', '\n'];
        return String::from(input.trim_end_matches(x));
    }

    /// Terminal input character check
    fn input_check(&mut self, input: String) -> Result<String, DataError> {
        if !input.chars().all(|c| {
            c.is_alphanumeric()
                || c == '.'
                || c == '+'
                || c == '-'
                || c == '|'
                || c == '@'
                || c == ' '
        }) || input == "".to_string()
        {
            Err(DataError::InvalidHeader {
                expected: ("specified command characters".to_string()),
                found: ("invalid character".to_string()),
            })
        } else {
            self.check.read_valid = true;
            Ok(input)
        }
    }

    /// Get instructions from the terminal
    pub fn terminal_read(&mut self, _prompt: &str) -> Result<String, DataError> {
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|_| DataError::InvalidHeader {
                expected: ("terminal input".to_string()),
                found: ("invalid input".to_string()),
            })?;

        // input parser and check
        input = self.input_parser(input);
        input = self.input_check(input)?;

        // input valid and apply it
        if let ConsoleStatus::InsAcqFromTerminal = self.current_status {
            self.current_ins = Some(input.clone());
        } else {
            self.current_cmd = Some(input.clone());
        }
        self.interact_prompt
            .mian_prompt
            .push_str(&format!("{} > ", input.clone()));
        // self.check.read_valid = true;

        Ok(input)
    }

    /// Get instructions from the file
    pub fn file_read(&mut self, _prompt: &str) -> Result<String, DataError> {
        let mut input = if let Some((_, input)) = if let ConsoleStatus::InsAcqFromFile = self.current_status {
            self.auto_exc.next_exc_ins.clone()
        } else {
            self.auto_exc.next_exc_cmd.clone()
        } {
            match input {
                GenericCmd::Character(v) => v,
                GenericCmd::Number(v) => v.to_string(),
            }
        } else {
            return Err(DataError::Redaction("error".to_string()))
        };
        let _ = self.file_poll();

        // input parser and check
        input = self.input_parser(input);
        input = self.input_check(input)?;

        // input valid and apply it
        if let ConsoleStatus::InsAcqFromTerminal = self.current_status {
            self.current_ins = Some(input.clone());
        } else {
            self.current_cmd = Some(input.clone());
        }
        self.interact_prompt
            .mian_prompt
            .push_str(&format!("{} > ", input.clone()));
        // self.check.read_valid = true;

        self.log(&input);

        Ok(input)
    }

    pub fn file_import(&mut self) -> Result<(), DataError> {
        self.auto_exc.file_address = Some(self.read("请输入文件地址")?);

        self.check.read_valid = true;
        let context = std::fs::read_to_string(&self.auto_exc.file_address.clone().unwrap())?;
        // self.auto_exc = toml::from_str::<ExcuteFile>(&context).map_err(|_| DataError::Unknown)?;
        self.auto_exc = match toml::from_str::<ExcuteFile>(&context) {
            Ok(v) => v,
            Err(_err_info) => {
                // println!("{:#?}", _err_info);
                return Err(DataError::Redaction("文件内容格式有误".to_string()));
            }
        };

        self.check.import_valid = true;
        self.check.file_valid = true;
        println!("{:#?}", self.auto_exc);
        self.refresh()?;

        // pre-population.
        let _ = self.file_poll();
        self.refresh()?;

        Ok(())
    }

    pub fn file_poll(&mut self) -> Result<String, DataError> {
        match self.auto_exc.next_exc_ins {
            None => {
                match self.auto_exc.next_exc_cmd {
                    None => {
                        if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(0) {
                            if let Some(ins) = &exc_assets.exc_ins {
                                self.auto_exc.next_exc_ins = Some((0, ins.clone()));
                            } else {
                                // No instruction error under instruction set
                            }
                        } else {
                            // No instruction set error
                        }
                    }
                    Some((_cmd_index, _)) => {
                        // Err
                    }
                }
            }
            Some((ins_index, _)) => {
                match self.auto_exc.next_exc_cmd {
                    None => {
                        // Go to the instruction set pointed to by the index.
                        if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(ins_index) {
                            if let Some(cmd) = exc_assets.sub_cmd_assets.get(0) {
                                // Get the first command in the instruction set
                                self.auto_exc.next_exc_cmd = Some((0, cmd.sub_cmd.clone()));
                            } else {
                                // No command in instruction set.
                                // Get the next instruction set instruction.
                                if let Some(exc_assets) =
                                    self.auto_exc.exc_ins_assets.get(ins_index + 1)
                                {
                                    if let Some(ins) = &exc_assets.exc_ins {
                                        self.auto_exc.next_exc_ins =
                                            Some((ins_index + 1, ins.clone()));
                                    } else {
                                        self.auto_exc.next_exc_ins = None;
                                        // Err
                                    }
                                } else {
                                    // End of file instruction set traversal.
                                    self.auto_exc.next_exc_ins = None;
                                }
                            }
                        } else {
                            self.auto_exc.next_exc_ins = None;
                            // Loss error.
                        }
                    }
                    Some((cmd_index, _)) => {
                        // Go to the instruction set pointed to by the index.
                        if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(ins_index) {
                            // Go to the command set pointed to by the index.
                            if let Some(cmd) = exc_assets.sub_cmd_assets.get(cmd_index + 1) {
                                // Get the next command in the instruction set
                                self.auto_exc.next_exc_cmd =
                                    Some((cmd_index + 1, cmd.sub_cmd.clone()));
                            } else {
                                // No command in instruction set.
                                // Get the next instruction set instruction.
                                if let Some(exc_assets) =
                                    self.auto_exc.exc_ins_assets.get(ins_index + 1)
                                {
                                    if let Some(ins) = &exc_assets.exc_ins {
                                        self.auto_exc.next_exc_ins =
                                            Some((ins_index + 1, ins.clone()));
                                        self.auto_exc.next_exc_cmd = None;
                                    } else {
                                        self.auto_exc.next_exc_ins = None;
                                        self.auto_exc.next_exc_cmd = None;
                                        // Err
                                    }
                                } else {
                                    // End of file instruction set traversal.
                                    self.auto_exc.next_exc_ins = None;
                                    self.auto_exc.next_exc_cmd = None;
                                }
                            }
                        } else {
                            self.auto_exc.next_exc_ins = None;
                            self.auto_exc.next_exc_cmd = None;
                            // Error.
                        }
                    }
                }
            }
        }

        if let Some((_, cmd)) = self.auto_exc.next_exc_cmd.clone() {
            match cmd {
                GenericCmd::Number(cmd) => Ok(cmd.to_string()),
                GenericCmd::Character(cmd) => Ok(cmd),
            }
        } else {
            if let Some((_, cmd)) = self.auto_exc.next_exc_ins.clone() {
                match cmd {
                    GenericCmd::Number(cmd) => Ok(cmd.to_string()),
                    GenericCmd::Character(cmd) => Ok(cmd),
                }
            } else {
                Err(DataError::Unknown)
            }
        }
    }

    pub fn read(&mut self, prompt: &str) -> Result<String, DataError> {
        // print prompt.
        self.log(&format!(
            "{}{}{}",
            self.interact_prompt.mian_prompt, self.interact_prompt.sub_prompt, prompt
        ));

        // File read command and terminal read command split.
        let cmd = match self.current_status {
            ConsoleStatus::InsAcqFromTerminal | ConsoleStatus::InsExecFromTerminal => {
                self.terminal_read(prompt)
            }
            ConsoleStatus::InsAcqFromFile | ConsoleStatus::InsExecFromFile => {
                self.file_read(prompt)
            }
            ConsoleStatus::Invaild => {
                self.refresh()?;
                return Err(DataError::InvalidHeader {
                    expected: ("determined console status".to_string()),
                    found: ("invalid status".to_string()),
                });
            }
        };

        match cmd {
            Ok(cmd) => {
                self.refresh()?;
                return Ok(cmd);
            }
            Err(err_info) => {
                self.err_log(&err_info);
                self.refresh()?;
                return Err(err_info);
            }
        }
    }

    /// Console state machine refresh
    pub fn refresh(&mut self) -> Result<(), DataError> {
        self.previous_status = self.current_status.clone();
        self.current_status = match self.current_status {
            ConsoleStatus::Invaild => ConsoleStatus::InsAcqFromTerminal,
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
            ConsoleStatus::InsAcqFromTerminal => {
                if self.check.read_valid {
                    ConsoleStatus::InsExecFromTerminal
                } else {
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
            ConsoleStatus::InsExecFromTerminal => match self.check.read_valid {
                true => {
                    if self.check.file_valid && self.check.import_valid {
                        self.prompt_clear();
                        ConsoleStatus::InsAcqFromFile
                    } else {
                        ConsoleStatus::InsExecFromTerminal
                    }
                }
                false => ConsoleStatus::Invaild,
            },
        };

        if let ConsoleStatus::Invaild = self.current_status {
            self.prompt_clear();
            self.previous_status = self.current_status.clone();
            self.current_status = ConsoleStatus::InsAcqFromTerminal;
        }

        if self.current_status != self.previous_status {
            println!(
                "
    + - - - - - - - - - + - - - - - - - - - - - +
    |   控制台当前状态  |  {:?}  
    + - - - - - - - - - + - - - - - - - - - - - +
        ",
                self.current_status
            );
        }

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
        println!("{}", log_info);
    }

    pub fn err_log<T>(&self, err_info: T)
    where
        T: fmt::Display + fmt::Debug,
    {
        println!("{:?}", err_info);
    }
}
