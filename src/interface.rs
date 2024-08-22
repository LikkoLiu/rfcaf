use std::fmt;

pub trait ConsoleLog {
    fn prompt_log(&self, log_info: &str) {
        println!("{}", log_info);
    }

    fn file_exc_log(&self, log_info: &str) {
        println!("{}", log_info);
    }

    fn err_log<T>(&self, err_info: T)
    where
        T: fmt::Display + fmt::Debug,
    {
        println!("{:?}", err_info);
    }

    fn err_invalid(&self) -> &'static str {
        "invalid input."
    }
}
