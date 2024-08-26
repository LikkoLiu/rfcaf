use std::sync::{Arc, Mutex};

use rfcaf::interface::ConsoleLog;
const ERR_VALID_INPUT: &'static str = "无效的输入";
extern crate rfcaf;
struct Log {
    err_info: &'static str,
}

impl Log {
    pub fn new() -> Self {
        Log {
            err_info: ERR_VALID_INPUT,
        }
    }
}

impl ConsoleLog for Log {
    fn err_invalid(&self) -> &'static str {
        self.err_info
    }
}

fn main() {
    let log = Log::new();
    let mut test = rfcaf::Console::new(Arc::new(Mutex::new(log)));
    test.setup();

    loop {
        if let Ok(cmd) = test.read("输入一条命令") {
            match cmd.as_str() {
                "R" | "r" => {
                    test.file_import_no_err();
                }
                _ => {}
            };
        }
    }
}
