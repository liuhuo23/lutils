use clap::{arg, command, ArgAction};
use lutils::logger::init;
use tracing::debug;

fn main() {
    let mut info = "error";
    let clash_cli = command!("clash cli")
        .arg(arg!(debug: -v --verbose "输出日志： -d info -dd debug").action(ArgAction::Count))
        .arg(arg!(install: -i --install "安装clash core").action(ArgAction::SetTrue))
        .get_matches();

    let count = clash_cli.get_count("debug");
    match count {
        1_u8 => info = "info",
        2_u8 => info = "debug",
        _ => {}
    };
    init(info).unwrap();
    debug!("日志级别：{}", info);
    if clash_cli.get_flag("install") {
        println!("安装clash");
    }
}
