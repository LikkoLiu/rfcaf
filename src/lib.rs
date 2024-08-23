/*
 * @Author: likkoliu
 * @Date: 2024-08-17 10:48:48
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-21 15:50:56
 * @Description:
 */
pub mod interface;
use crate::interface::ConsoleLog;
use serde_derive::Deserialize;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use toml;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("data error")]
    Other(#[from] io::Error), // Convert other error types.
    #[error("{0}")]
    Redaction(String), // error action.
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String }, // dismatch expect input.
    #[error("unknown data error")]
    Unknown,
}

/// Supported file-command data types.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum GenericCmd {
    Number(usize),
    Character(String),
}

/// Console Status.
#[derive(Debug, Clone, PartialEq)]
enum ConsoleStatus {
    InsAcqFromFile,     // instruction acquisition from file status.
    InsAcqFromTerminal, // instruction acquisition from terminal status.

    InsExecFromFile,     // instruction execution from file status.
    InsExecFromTerminal, // instruction execution from terminal status.

    Invaild, // invaild state.
}

#[derive(Deserialize, Debug)]
struct ValidCheck {
    read_valid: bool,   // Command read valid.
    import_valid: bool, // Import file is valid.
    file_valid: bool,   // The file address has been obtained.
}

/// Command line echo prompt.
#[derive(Debug)]
struct ConsolePrompt {
    mian_prompt: String,
    sub_prompt: String,
}

/// Automation command execution file config.
#[derive(Deserialize, Debug)]
struct ExecuteFile {
    file_address: Option<String>, // <populated by file_import> automatic execution command file address.

    exc_ins_assets: Vec<ExecuteAssets>, // <collections> automatically execute instructions and command assets.
    cycle_times: Option<usize>, // <option, default one time> automatic execution cycle times.

    next_exc_ins: Option<(usize, GenericCmd)>, // <populated by file_poll> next automatic execution instruction.
    next_exc_cmd: Option<(usize, GenericCmd)>, // <populated by file_poll> next auto-execute command.
}

#[derive(Deserialize, Debug)]
struct ExecuteAssets {
    exc_ins: GenericCmd, // <required> Automatic execution instruction.
    sub_cmd_assets: Option<Vec<SubCmd>>, // <option> Auto-execute command assets.
}

#[derive(Deserialize, Debug)]
struct SubCmd {
    sub_cmd: GenericCmd,
}

#[derive(Debug)]
struct Status {
    current: ConsoleStatus,
    previous: ConsoleStatus,
}

#[derive(Debug)]
pub struct Console<T>
where
    T: ConsoleLog,
{
    status: Status,
    check: ValidCheck,

    interact: ConsolePrompt,
    log: Arc<Mutex<T>>,
    pub _input_invalid: &'static str,

    auto_exc: ExecuteFile,

    current_ins: Option<String>, // currently executing instruction.
    current_cmd: Option<String>, // currently executing command.
}

impl<T> Console<T>
where
    T: ConsoleLog,
{
    pub fn new(log: Arc<Mutex<T>>) -> Self {
        let invalid_info = match log.lock().map_err(|_| {
            DataError::Redaction("log information prints mutex acquisition failure.".to_string())
        }) {
            Ok(log) => log.err_invalid(),
            Err(_err_info) => {
                panic!("{}", _err_info);
            }
        };

        Console {
            status: Status {
                current: ConsoleStatus::Invaild,
                previous: ConsoleStatus::Invaild,
            },
            check: ValidCheck {
                read_valid: false,
                import_valid: false,
                file_valid: false,
            },

            interact: ConsolePrompt {
                mian_prompt: String::from("> "),
                sub_prompt: String::from(""),
            },
            log: log,
            _input_invalid: invalid_info,

            auto_exc: ExecuteFile {
                file_address: None,
                exc_ins_assets: Vec::new(),
                cycle_times: None,
                next_exc_ins: None,
                next_exc_cmd: None,
            },

            current_ins: None,
            current_cmd: None,
        }
    }

    /// initialize after creating the console object to refresh the state machine.
    pub fn setup(&mut self) {
        let _ = self.refresh();
    }

    /// called when a set of instructions has completed execution.
    pub fn taildowm(&mut self) {
        let _ = self.refresh();
    }

    /// input character parser.
    fn input_parser(&self, input: String) -> String {
        let x: &[_] = &['\r', '\n'];
        return String::from(input.trim_end_matches(x));
    }

    /// input character check.
    fn input_check(&mut self, input: &str) -> Result<bool, DataError> {
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
                found: ("invalid characters".to_string()),
            })
        } else {
            Ok(true)
        }
    }

    /// get instructions from the terminal.
    fn terminal_read(&mut self, _prompt: &str) -> Result<String, DataError> {
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|_| DataError::InvalidHeader {
                expected: ("terminal input".to_string()),
                found: ("invalid input".to_string()),
            })?;

        // input parser and check.
        input = self.input_parser(input);
        self.check.read_valid = self.input_check(&input)?;

        // input valid and apply it.
        if let ConsoleStatus::InsAcqFromTerminal = self.status.current {
            self.current_ins = Some(input.clone());
        } else {
            self.current_cmd = Some(input.clone());
        }
        self.interact
            .sub_prompt
            .push_str(&format!("{} > ", input.clone()));

        // terminal command execution output.
        match self.log.lock().map_err(|_| {
            DataError::Redaction("log information prints mutex acquisition failure.".to_string())
        }) {
            Ok(log) => log.terminal_exc_log(&input),
            Err(_err_info) => {
                panic!("{}", _err_info);
            }
        }

        Ok(input)
    }

    /// Get instructions from the file.
    fn file_read(&mut self, _prompt: &str) -> Result<String, DataError> {
        let mut input = if let Some((_, input)) =
            if let ConsoleStatus::InsAcqFromFile = self.status.current {
                self.auto_exc.next_exc_ins.clone()
            } else {
                self.auto_exc.next_exc_cmd.clone()
            } {
            match input {
                GenericCmd::Character(v) => v,
                GenericCmd::Number(v) => v.to_string(),
            }
        } else {
            return Err(DataError::Redaction(
                "no executable instructions or commands.".to_string(),
            ));
        };
        let _ = self.file_poll();

        // input parser and check.
        input = self.input_parser(input);
        self.check.read_valid = self.input_check(&input)?;

        // input valid and apply it.
        if let ConsoleStatus::InsAcqFromTerminal = self.status.current {
            self.current_ins = Some(input.clone());
        } else {
            self.current_cmd = Some(input.clone());
        }
        self.interact
            .mian_prompt
            .push_str(&format!("{} > ", input.clone()));

        // automatic file command execution output.
        match self.log.lock().map_err(|_| {
            DataError::Redaction("log information prints mutex acquisition failure.".to_string())
        }) {
            Ok(log) => log.file_exc_log(&input),
            Err(_err_info) => {
                panic!("{}", _err_info);
            }
        }

        Ok(input)
    }

    pub fn file_import(&mut self) -> Result<(), DataError> {
        // clear the saved command set.
        Console::exc_clear(self);

        self.auto_exc.file_address = Some(self.read("请输入文件地址")?);
        self.check.read_valid = true; // nead re-set in file_import.
        self.check.file_valid = true;
        let context = std::fs::read_to_string(&self.auto_exc.file_address.clone().unwrap())?;
        self.auto_exc = match toml::from_str::<ExecuteFile>(&context) {
            Ok(v) => v,
            Err(_err_info) => {
                return Err(DataError::Redaction(format!(
                    "{} {}  {}  {}",
                    "文件内容格式有误，检查文件内容是否满足：",
                    "- 文件涉及测试组 执行次数 <可选，若未输入默认执行一次>",
                    "- 单次测试 主指令 <必须>",
                    "- 单次测试 子命令/子命令集 <可选>"
                )));
            }
        };

        // pre-population.
        self.file_poll()?;
        self.check.import_valid = true;
        // println!("{:#?}", self.auto_exc);

        self.refresh()?;
        Ok(())
    }

    pub fn file_import_no_err(&mut self) {
        match self.file_import() {
            Ok(_) => {}
            Err(err_info) => {
                // if the log mutex acquisition fails, it will panic automatically.
                match self.log.lock().map_err(|_| {
                    DataError::Redaction(
                        "log information prints mutex acquisition failure.".to_string(),
                    )
                }) {
                    Ok(log) => log.err_log(&err_info),
                    Err(_err_info) => {
                        panic!("{}", _err_info);
                    }
                }
            }
        }
    }

    fn file_poll(&mut self) -> Result<String, DataError> {
        match self.auto_exc.next_exc_ins {
            None => match self.auto_exc.next_exc_cmd {
                None => {
                    if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(0) {
                        self.auto_exc.next_exc_ins = Some((0, exc_assets.exc_ins.clone()));
                    } else {
                        Console::exc_clear(self);
                        return Err(DataError::Redaction(format!(
                            "获取第一条主指令集失败，文件导入的指令集内容被污染，请重新导入文件。"
                        )));
                    }
                }
                Some((_cmd_index, _)) => {
                    Console::exc_clear(self);
                    return Err(DataError::Redaction(format!(
                        "子命令的主指令意外丢失，请重新导入文件开始测试。"
                    )));
                }
            },
            Some((ins_index, _)) => {
                match self.auto_exc.next_exc_cmd {
                    None => {
                        // Go to the instruction set pointed to by the index.
                        if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(ins_index) {
                            if let Some(sub_cmd_assets) = &exc_assets.sub_cmd_assets {
                                if let Some(cmd) = sub_cmd_assets.get(0) {
                                    // Get the first command in the instruction set
                                    self.auto_exc.next_exc_cmd = Some((0, cmd.sub_cmd.clone()));
                                }
                            } else {
                                // No command in instruction set.
                                // Get the next instruction set instruction.
                                if let Some(exc_assets) =
                                    self.auto_exc.exc_ins_assets.get(ins_index + 1)
                                {
                                    self.auto_exc.next_exc_ins =
                                        Some((ins_index + 1, exc_assets.exc_ins.clone()));
                                } else {
                                    // End of file instruction set traversal.
                                    self.auto_exc.next_exc_ins = None;

                                    // cycle judgment
                                    if match self.auto_exc.cycle_times {
                                        Some(cycle_times) => {
                                            self.auto_exc.cycle_times = Some(cycle_times - 1);
                                            cycle_times - 1
                                        }
                                        None => 0,
                                    } != 0
                                    {
                                        if let Some(exc_assets) =
                                            self.auto_exc.exc_ins_assets.get(0)
                                        {
                                            self.auto_exc.next_exc_ins =
                                                Some((0, exc_assets.exc_ins.clone()));
                                        } else {
                                            Console::exc_clear(self);
                                            return Err(DataError::Redaction(format!(
                                                "获取第一条主指令集失败，文件导入的指令集内容被污染，请重新导入文件。"
                                            )));
                                        }
                                    }
                                }
                            }
                        } else {
                            Console::exc_clear(self);
                            return Err(DataError::Redaction(format!(
                                "读取指定主指令集失败，请重新导入文件开始测试。"
                            )));
                        }
                    }
                    Some((cmd_index, _)) => {
                        // Go to the instruction set pointed to by the index.
                        if let Some(exc_assets) = self.auto_exc.exc_ins_assets.get(ins_index) {
                            if let Some(sub_cmd_assets) = &exc_assets.sub_cmd_assets {
                                // Go to the command set pointed to by the index.
                                if let Some(cmd) = sub_cmd_assets.get(cmd_index + 1) {
                                    // Get the next command in the instruction set
                                    self.auto_exc.next_exc_cmd =
                                        Some((cmd_index + 1, cmd.sub_cmd.clone()));
                                } else {
                                    // No command in instruction set.
                                    // Get the next instruction set instruction.
                                    if let Some(exc_assets) =
                                        self.auto_exc.exc_ins_assets.get(ins_index + 1)
                                    {
                                        self.auto_exc.next_exc_ins =
                                            Some((ins_index + 1, exc_assets.exc_ins.clone()));
                                    } else {
                                        // End of file instruction set traversal.
                                        self.auto_exc.next_exc_ins = None;
                                        self.auto_exc.next_exc_cmd = None;

                                        // cycle judgment
                                        if match self.auto_exc.cycle_times {
                                            Some(cycle_times) => {
                                                self.auto_exc.cycle_times = Some(cycle_times - 1);
                                                cycle_times - 1
                                            }
                                            None => 0,
                                        } != 0
                                        {
                                            if let Some(exc_assets) =
                                                self.auto_exc.exc_ins_assets.get(0)
                                            {
                                                self.auto_exc.next_exc_ins =
                                                    Some((0, exc_assets.exc_ins.clone()));
                                            } else {
                                                Console::exc_clear(self);
                                                return Err(DataError::Redaction(format!(
                                                    "获取第一条主指令集失败，文件导入的指令集内容被污染，请重新导入文件。"
                                                )));
                                            }
                                        }
                                    }
                                }
                            } else {
                                Console::exc_clear(self);
                                return Err(DataError::Redaction(format!(
                                    "指定主指令集的子命令集意外丢失，请重新导入文件开始测试。"
                                )));
                            }
                        } else {
                            Console::exc_clear(self);
                            return Err(DataError::Redaction(format!(
                                "子命令的主指令意外丢失，请重新导入文件开始测试。"
                            )));
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
        match self.log.lock().map_err(|_| {
            DataError::Redaction("log information prints mutex acquisition failure.".to_string())
        }) {
            Ok(log) => {
                if prompt == "" {
                    log.prompt_log(&format!(
                        "{}{}",
                        self.interact.mian_prompt, self.interact.sub_prompt
                    ))
                } else {
                    log.prompt_log(&format!(
                        "{}{}\r\n{}",
                        self.interact.mian_prompt, self.interact.sub_prompt, prompt
                    ))
                }
            }
            Err(_err_info) => {
                panic!("{}", _err_info);
            }
        }

        // File read command and terminal read command split.
        let cmd = match self.status.current {
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
                self.refresh()?;
                return Err(err_info);
            }
        }
    }

    pub fn read_no_err(&mut self, prompt: &str) -> String {
        match self.read(prompt) {
            Ok(input) => input,
            Err(err_info) => {
                // If the log mutex acquisition fails, it will panic automatically.
                match self.log.lock().map_err(|_| {
                    DataError::Redaction(
                        "log information prints mutex acquisition failure.".to_string(),
                    )
                }) {
                    Ok(log) => log.err_log(&err_info),
                    Err(_err_info) => {
                        panic!("{}", _err_info);
                    }
                }
                "".to_string()
            }
        }
    }

    /// Console state machine refresh
    fn refresh(&mut self) -> Result<(), DataError> {
        self.status.previous = self.status.current.clone();
        self.status.current = match self.status.current {
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
                    if self.check.file_valid {
                        if let Some(_) = self.auto_exc.next_exc_ins {
                            self.prompt_clear();
                            ConsoleStatus::InsAcqFromFile
                        } else {
                            ConsoleStatus::Invaild
                        }
                    } else {
                        ConsoleStatus::InsExecFromTerminal
                    }
                }
                false => ConsoleStatus::Invaild,
            },
        };

        if let ConsoleStatus::Invaild = self.status.current {
            self.prompt_clear();
            self.status.previous = self.status.current.clone();
            self.status.current = ConsoleStatus::InsAcqFromTerminal;
        }

        if self.status.current != self.status.previous {
            println!(
                "
    + - - - - - - - - - + - - - - - - - - - - - - - - - - - - - - +
    |   控制台当前状态  |  {:?} -> {:?}   
    + - - - - - - - - - + - - - - - - - - - - - - - - - - - - - - +",
                self.status.previous, self.status.current
            );
        }

        self.check_reset();
        Ok(())
    }

    /// Clear the console command cache
    fn prompt_clear(&mut self) {
        self.interact.mian_prompt = String::from("> ");
        self.interact.sub_prompt = String::from("");
    }

    fn exc_clear(&mut self) {
        self.auto_exc.file_address = None;
        self.auto_exc.exc_ins_assets.clear();
        self.auto_exc.cycle_times = None;
        self.auto_exc.next_exc_cmd = None;
        self.auto_exc.next_exc_ins = None;
    }

    fn check_reset(&mut self) {
        self.check.file_valid = false;
        self.check.import_valid = false;
        self.check.read_valid = false;
    }
}
