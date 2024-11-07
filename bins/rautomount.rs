use clap::{arg, command, Arg, ArgAction};
use lutils::logger::init;
use lutils::mount::BlkidList;
use std::path::Path;
use std::process::{exit, Command};
use tracing::{debug, info};
fn main() {
    let mut info = "error";
    let cmd_match = command!("automount")
        .version("0.1.0")
        .author("liuhuo")
        .bin_name("automount")
        .arg(arg!(debug: -v --verbose "输出日志： -d info -dd debug").action(ArgAction::Count))
        .subcommand(
            command!("mount")
                .about("挂载设备")
                .arg(arg!(all: -a --all "加载所有label不为空的设备").action(ArgAction::SetTrue))
                .arg(
                    Arg::new("device")
                        .help("<DEV> 设备名称")
                        .action(ArgAction::Set),
                ),
        )
        .arg(arg!(list: -l --list "显示所有label不为空的设备").action(ArgAction::SetTrue))
        .get_matches();
    let count = cmd_match.get_count("debug");
    match count {
        1_u8 => info = "info",
        2_u8 => info = "debug",
        _ => {}
    };
    init(info).unwrap();
    debug!("日志级别：{}", info);
    let stdout = Command::new("blkid").arg("-d").output().unwrap();
    let res = String::from_utf8(stdout.stdout).unwrap();
    let blkid_list = BlkidList::new(&res);
    if let Some(sub_match) = cmd_match.subcommand_matches("mount") {
        info!("挂载设备");
        if sub_match.get_flag("all") {
            info!("命令：all, 加载所有label不为空的设备");
            for item in blkid_list.get_label_device() {
                let path_str = format!("/mnt/{}", item.label);
                let path = Path::new(&path_str);
                if !path.exists() {
                    info!("创建目录：{}", path.display());
                    std::fs::create_dir(path).unwrap();
                }
                let output = item.mount(&path_str);
                if output.status.success() {
                    println!("挂载设备：{} to {}", item.name, path_str);
                } else {
                    eprint!(
                        "挂载失败：{}, 错误：{}\n",
                        output.status,
                        String::from_utf8(output.stderr).unwrap()
                    );
                    continue;
                }
            }
            exit(0);
        }
        if let Some(device) = sub_match.get_one::<String>("device") {
            info!("加载设备：{}", device);
            if let Some(dev) = blkid_list.find_device(&device) {
                let path_str = format!("/mnt/{}", dev.label);
                let path = Path::new(&path_str);
                if !path.exists() {
                    info!("创建目录：{}", path.display());
                    std::fs::create_dir(path).unwrap();
                }
                info!("挂载设备：{}", dev.name);
                let output = dev.mount(&path_str);
                if output.status.success() {
                    println!("挂载设备：{} to {}", dev.name, path_str);
                } else {
                    eprint!(
                        "挂载失败：{}, 错误：{}\n",
                        output.status,
                        String::from_utf8(output.stderr).unwrap()
                    );
                    exit(-1);
                }
            } else {
                eprint!("没有找到设备：{}\n", device);
                exit(-1);
            }
            exit(0);
        } else {
            info!("或许你可以添加 --all 或者 <DEV>");
        }
        exit(0);
    }
    if cmd_match.get_flag("list") {
        info!("显示所有label不为空的设备");
        for item in blkid_list.get_label_device() {
            println!("{:?}", item);
        }
    } else {
        info!("显示所有设备");
        println!("{}", blkid_list);
    }

    exit(0);
}
