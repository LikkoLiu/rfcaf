/*
 * @Author: liujiajin
 * @Date: 2024-08-17 10:58:54
 * @LastEditors: Please set LastEditors
 * @LastEditTime: 2024-08-19 17:39:49
 * @Description: 
 */
extern crate rfcaf;

fn main() {
    let mut test = rfcaf::Console::new();
    println!("{:?}", test);
    // if let Err(err_info) = test.file_import("example.toml") {
    //     println!("{:?}", err_info);
    // }
    test.refresh();
    
    loop {

        test.read("输入一条命令");

    }

    println!("hello world!");
}
