use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use std::io::{self, BufRead, BufReader};


#[derive(Debug)]
pub enum DiskCommand{
    Mount {
        loop_device: PathBuf,
        target: PathBuf
    },
    Umount {
        target: PathBuf
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FileSystem {
    Ext4,
    // Xfs,
    // Vfat,
}

#[derive(Debug)]
pub struct LoopDevice {
    device_path: PathBuf
}

#[derive(Debug)]
pub struct FileArgs {
    pub path: PathBuf,
    pub name: String,
    pub size: u64
}


impl DiskCommand {
    
    pub fn new_mount(loop_device: String, target: String) -> Self {
        Self::Mount {
            loop_device: PathBuf::from(loop_device),
            target: PathBuf::from(target)
        }
    }

    pub fn new_umount(target: String) -> Self {
        Self::Umount {
            target: PathBuf::from(target)
        }
    }

    pub fn execute(&self) {
        match self {
            DiskCommand::Mount { loop_device, target } => {
                println!("Execute: mount {:?} {:?}", loop_device, target);

                // Mount loop device
                let mount_result = Command::new("mount")
                    .arg(loop_device)
                    .arg(target)
                    .output()
                    .map_err(|e| format!("Failed to execute mount command: {}", e));

                match mount_result {
                    Ok(output) => {
                        if output.status.success() {
                            println!("-> Successfully mounted {:?} to {:?}", loop_device, target);
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            eprintln!("Error: mount command failed (exit code {}). Error output: {}", output.status.code().unwrap_or(-1), stderr);
                        }
                    },
                    Err(e) => eprintln!("Error: {}", e)
                }

            }
            DiskCommand::Umount { target } => {
                println!("Execute: umount {:?}", target);

                // Umount first
                let umount_result = Command::new("umount")
                    .arg(target)
                    .output()
                    .map_err(|e| format!("Failed to execute umount: {}", e));

                match umount_result {
                    Ok(output) => {
                        if !output.status.success() {
                            eprintln!("Warning could not unmount: {:?}. It might not have been mounted or permission were insufficient.", target);
                        } else {
                            println!("-> Successfully unmounted {:?}", target);
                        }
                    },
                    Err(e) => eprintln!("Error during ummount attempt: {}", e)
                }

            }
        }
    }

}


impl FileSystem {
    // Method to get the mkfs utility string.
    fn mkfs_command(&self) -> &'static str {
        match self {
            FileSystem::Ext4 => "mkfs.ext4",
            // FileSystem::Xfs => "mkfs.xfs",
            // FileSystem::Vfat => "mkfs.vfat",
        }
    }
}


impl LoopDevice {

    pub fn new(path: String) -> Self {

        Self { device_path: PathBuf::from(path) }

    }

    pub fn fs_status(&self) -> bool {

        let status = Command::new("blkid")
            .arg("-p")
            .arg(&self.device_path)
            .status();

        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    println!("File system on {:?} not exist.", self.device_path);
                    true
                } else if exit_status.code() == Some(2) {
                    // low-level probing found no valid signatures
                    println!("On {:?} file system not exist.", self.device_path);
                    false
                } else {
                    println!("Permission denied (run with sudo).");
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to run blkid: {}", e);
                false
            }
        }
    }

    pub fn format_device(&self, fstype: FileSystem) -> Result<(), String> {
        let cmd = fstype.mkfs_command();

        let output = Command::new(cmd)
            .arg(&self.device_path)
            .output()
            .map_err(|e| format!("Failed to write command {}: {}", cmd, e))?;

        if output.status.success() {
            println!("Device {:?} is formated as {:?}", self.device_path, fstype);
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr).into_owned();
            Err(format!("Failed to format: {}", error_msg))
        }
    }

    pub fn ensure_formatted(&self, default_fs: FileSystem) {
        if !self.fs_status() {
            if let Err(e) = self.format_device(default_fs) {
                eprintln!("Критическая ошибка: {}", e);
            }
        } else {
            println!("Пропуск форматирования для {:?}", self.device_path);
        }
    }

}


/// Create a small file to act the disk image
pub fn create_image_file(args: FileArgs) -> Result<(), std::io::Error> {

    println!("Create image file: {}", args.name);

    let full_path = args.path.join(&args.name);

    // create file in target directory
    let file = std::fs::File::create(&full_path)?;

    println!("Path: {}", &full_path.display());
 
    println!("Size: {}", &args.size);

    // Write zero bytes to simulate an image. 32MB minimum size for the ext4 FS 
    file.set_len(&args.size * 1024 * 1024)?;

    Ok(())
}


// Get name loop device from path of image file
pub fn get_device_name(image: &str) -> Result<String, std::io::Error> {

    // Run losetup command, we need to parse the output to find the device name
    let losetup_list_devices = Command::new("losetup")
        .arg("-a")
        .output()?;

    let stdout_string = String::from_utf8(losetup_list_devices.stdout).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
    let loop_device = stdout_string
        .lines()
        .find(|line| line.contains(&image))
        .and_then(|line| line.split_once(':').map(|(dev, _)| dev.trim()));

    // return device path or error
    match loop_device {
        Some(device) => Ok(device.to_string()),
        None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Could not determine the loop device path for image path: {}", image)))
    }

}


pub fn get_loop_device(mount_point: &str) -> std::io::Result<Option<String>> {
    let file = File::open("/proc/mounts")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        // In /proc/mounts: parts[0] - device, parts[1] - mount point
        if parts.len() >= 2 && parts[1] == mount_point {
            return Ok(Some(parts[0].to_string()));
        }
    }
    Ok(None)
}

pub fn get_mount_point(device_path: &str) -> Option<PathBuf> {
    // Открываем файл со списком текущих монтирований в Linux
    let file = File::open("/proc/mounts").ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().flatten() {
        // Разделяем строку по пробелам
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        // Структура строки: [0] девайс, [1] точка монтирования, [2] тип ФС...
        if parts.len() >= 2 && parts[0] == device_path {
            return Some(PathBuf::from(parts[1]));
        }
    }
    None // Если устройство не примонтировано
}