use std::fmt;
use std::fs::File;
use std::io::Read;
use std::process::Output;

use anyhow::anyhow;
use anyhow::Result;
use regex::Regex;
use serde::Deserializer;
use serde::{Deserialize, Serialize};

use crate::error::UtilsError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    #[serde(rename = "maj:min")]
    pub maj_min: String,
    #[serde(rename = "size", deserialize_with = "deserialize_size")]
    pub size: u64,
    pub ro: bool,
    #[serde(rename = "type")]
    pub d_type: String,
    pub mountpoints: Vec<Option<String>>,
    #[serde(default = "Vec::new")]
    children: Vec<Device>,
}

fn deserialize_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    Ok(capacity(&name))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub blockdevices: Vec<Device>,
}

impl Block {
    pub fn new(json_str: &str) -> Self {
        serde_json::from_str(json_str).unwrap()
    }
}

// 将类似 458。7G 转成单位字节
pub fn capacity<'a>(size: &'a str) -> u64 {
    let unit = size.chars().last().unwrap();
    let res = match unit {
        'T' => size[..size.len() - 1].parse::<f64>().unwrap() * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        'G' => size[..size.len() - 1].parse::<f64>().unwrap() * 1024.0 * 1024.0 * 1024.0,
        'M' => size[..size.len() - 1].parse::<f64>().unwrap() * 1024.0 * 1024.0,
        'K' => size[..size.len() - 1].parse::<f64>().unwrap() * 1024.0,
        _ => size.parse::<f64>().unwrap(),
    };
    res as u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blkid {
    pub name: String,
    pub label: String,
    pub uuid: String,
    #[serde(rename = "type")]
    pub d_type: String,
    pub partuuid: String,
    #[serde(skip)]
    pub is_mount: bool,
}

impl fmt::Display for Blkid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\t", self.name)?;
        write!(f, "{}\t", self.label)?;
        write!(f, "{}\t", self.uuid)?;
        write!(f, "{}\t", self.d_type)?;
        write!(f, "{}\t", self.partuuid)?;
        write!(f, "{}\n", self.is_mount)?;
        Ok(())
    }
}

impl Blkid {
    pub fn new(value: &str) -> Result<Self> {
        let mut blkid = Blkid {
            name: "".to_string(),
            label: "".to_string(),
            uuid: "".to_string(),
            d_type: "".to_string(),
            partuuid: "".to_string(),
            is_mount: false,
        };
        let re = Regex::new(
            "^(.*): *(LABEL=\"(.*?)\")?.*?UUID=\"(.*?)\".*?TYPE=\"(.*?)\".*?(PARTUUID=\"(.*)\")?$",
        );
        let re = match re {
            Ok(re) => re,
            Err(err) => return Err(anyhow!(UtilsError::BlkidError(err.to_string()))),
        };
        if let Some(caps) = re.captures(value) {
            blkid.name = caps[1].to_string();
            blkid.is_mount = check_mount(&blkid.name);
            if let Some(_) = caps.get(2) {
                blkid.label = caps[3].to_string();
            }
            blkid.uuid = caps[4].to_string();
            blkid.d_type = caps[5].to_string();
            if let Some(_) = caps.get(6) {
                blkid.partuuid = caps[7].to_string();
            }
        } else {
            return Err(anyhow!(UtilsError::BlkidError(format!(
                "no match: {}",
                value
            ))));
        }
        Ok(blkid)
    }

    pub fn mount(&self, path: &str) -> Output {
        // sudo mount -o rw -t ntfs /dev/sdb1 /mnt/ntfs
        let mut args = vec![];
        match self.d_type.as_str() {
            "ntfs" => {
                args.push("-t");
                args.push("ntfs");
            }
            _ => {}
        };
        let output = std::process::Command::new("mount")
            .arg("-o")
            .arg("rw")
            .args(args)
            .arg(self.name.clone())
            .arg(path)
            .output()
            .unwrap();
        output
    }

    pub fn umount(&self, path: &str) -> Output {
        // sudo umount /dev/sdb1
        let output = std::process::Command::new("umount")
            .arg(path)
            .output()
            .unwrap();
        output
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlkidList {
    pub blkids: Vec<Blkid>,
}

impl BlkidList {
    pub fn new(output_str: &str) -> Self {
        let mut blkids: Vec<Blkid> = Vec::new();
        for line in output_str.split("\n") {
            if let Ok(blkid) = Blkid::new(line) {
                blkids.push(blkid);
            }
        }
        Self { blkids }
    }

    pub fn get_label_device(&self) -> Vec<&Blkid> {
        let mut res = vec![];
        for item in self.blkids.iter() {
            if !item.is_mount && item.label.len() > 0 {
                res.push(item);
            }
        }
        return res;
    }

    pub fn find_device(&self, label: &str) -> Option<&Blkid> {
        for item in self.blkids.iter() {
            if item.label.len() > 0 && item.label == label && !item.is_mount {
                return Some(item);
            }
        }
        return None;
    }
}

impl fmt::Display for BlkidList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name     |")?;
        write!(f, "Label    |")?;
        write!(f, "UUID     |")?;
        write!(f, "Type     |")?;
        write!(f, "PartUUID |")?;
        write!(f, "IsMount  |\n")?;
        for item in self.blkids.iter() {
            writeln!(f, "{}", item)?;
        }
        Ok(())
    }
}

pub fn check_mount(device: &str) -> bool {
    let mut fs = File::open("/proc/mounts").unwrap();
    let mut buf = String::new();
    fs.read_to_string(&mut buf).unwrap();
    let re = Regex::new(&format!("^{}.*", device)).unwrap();
    for line in buf.lines() {
        if re.is_match(line) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::mount::{self, Blkid};
    use std::ffi::CString;

    use super::Device;

    #[test]
    fn str_to_struct() {
        let json_str = r#"
{
    "name": "nvme0n1",
    "maj:min": "259:0",
    "rm": false,
    "size": "465.8G",
    "ro": false,
    "type": "disk",
    "mountpoints": [null]
}
        "#;
        let device: Device = serde_json::from_str(&json_str).unwrap();
        println!("{:?}", device);
    }

    #[test]
    fn test_capacity() {
        assert_eq!(mount::capacity("100G"), 100 * 1024 * 1024 * 1024);
        assert_eq!(mount::capacity("100M"), 100 * 1024 * 1024);
        assert_eq!(mount::capacity("100"), 100);
    }

    #[test]
    fn test_to_blkid() {
        let value = r#"/dev/mapper/openeuler-swap: UUID="75d304ca-20a0-47d2-bf29-da460789c643" BLOCK_SIZE="512  TYPE="swap""#;
        let blkid = super::Blkid::new(value).unwrap();
        assert_eq!(blkid.name, "/dev/mapper/openeuler-swap");
        assert_eq!(blkid.uuid, "75d304ca-20a0-47d2-bf29-da460789c643");
        assert_eq!(blkid.d_type, "swap");
        assert_eq!(blkid.partuuid, "");
        let value = r#"/dev/nvme0n1p3: UUID="SB2XCA-H6oF-tZVR-TYkd-wVBC-Hee6-t4QUg1" TYPE="LVM2_member" PARTUUID="67426631-3f86-4de0-9d16-ca5fbd540604""#;
        let blkid = super::Blkid::new(value).unwrap();
        assert_eq!(blkid.name, "/dev/nvme0n1p3");
        assert_eq!(blkid.uuid, "SB2XCA-H6oF-tZVR-TYkd-wVBC-Hee6-t4QUg1");
        assert_eq!(blkid.d_type, "LVM2_member");
        assert_eq!(blkid.partuuid, "67426631-3f86-4de0-9d16-ca5fbd540604");
        let value = r#"/dev/sda2: LABEL="系统" BLOCK_SIZE="512" UUID="F4C41C2EC41BF21A" TYPE="ntfs" PARTLABEL="Basic data partition" PARTUUID="15565c7f-ea2b-41ed-b159-fe00ad7991f0""#;
        let blkid = Blkid::new(value).unwrap();
        assert_eq!(blkid.name, "/dev/sda2");
        assert_eq!(blkid.label, "系统");
        assert_eq!(blkid.uuid, "F4C41C2EC41BF21A");
        assert_eq!(blkid.d_type, "ntfs");
        assert_eq!(blkid.partuuid, "15565c7f-ea2b-41ed-b159-fe00ad7991f0");
        assert_eq!(blkid.is_mount, false);
    }

    #[test]
    fn test_mount_point() {
        let name = CString::new("/dev/sdb4").unwrap();
        let char = name.as_ptr();
        let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
        let res = unsafe { libc::stat(char, &mut statbuf) };
        if res == 0 {
            println!("{}", statbuf.st_rdev);
            println!("{}", statbuf.st_uid);
            println!("{}", statbuf.st_ino);
        }
    }

    #[test]
    fn test_check_mount() {
        assert_eq!(mount::check_mount("/dev/sdb4"), true);
        assert_eq!(mount::check_mount("/dev/sdb3"), false)
    }
}
