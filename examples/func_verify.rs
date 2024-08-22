/*
 * @Author: liujiajin
 * @Date: 2024-08-17 10:58:54
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-21 14:33:47
 * @Description:
 */
extern crate rfcaf;

fn main() {
    let mut test = rfcaf::Console::new();
    test.setup();

    loop {
        if let Ok(cmd) = test.read("输入一条命令") {
            match cmd.as_str() {
                "R" | "r" => {
                    if let Err(err_info) = test.file_import() {
                        test.err_log(err_info);
                    }
                }
                _ => {}
            };
        }
    }
}
