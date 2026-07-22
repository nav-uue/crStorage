use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::{self, File};
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
    pub user: String,
    pub image: String,
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
                eprintln!("Critical error: {}", e);
            }
        } else {
            println!("Skipping formatting for {:?}", self.device_path);
        }
    }

}


/// Create a small file to act the disk image
pub fn create_image_file(args: FileArgs) -> Result<(), std::io::Error> {

    // Convert a string to a path
    let full_path = Path::new(&args.image);

    let img_path = match full_path.parent() {
        Some(path) if path.to_string_lossy().is_empty() || path.to_string_lossy() == "." => format!("/home/{}", args.user),
        Some(path) => path.to_string_lossy().into_owned(),
        None => format!("/home/{}", args.user)
    };

    let img_name = match full_path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => format!("{}.img", args.user)
    };

    // Create all directories in the path
    if !Path::new(&img_path).is_dir() {
        fs::create_dir_all(&img_path).expect("Failed create image path");
    };

    // Create a new path after validation
    let new_path_str = format!("{}/{}", img_path, img_name);
    let new_path = Path::new(&new_path_str);

    match new_path.exists() {
        true => {

            let parts: Vec<&str> = img_name.split(".").collect();

            let base_name = parts.get(0).unwrap_or(&"user").to_string();
            let extension = parts.get(1).unwrap_or(&"img").to_string();

            let mut counter = 0;
            let mut final_path = String::new();

            // Loop until an available name is found
            loop {
                // Generate names sequentially: "name.img" for 0, "name_1.img" for 1, etc.
                let candidate_name = match counter {
                    0 => format!("{}.{}", base_name, extension),
                    _ => format!("{}_{}.{}", base_name, counter, extension),
                };

                // Check if the path exists
                match Path::new(&candidate_name).exists() {
                    true => {
                        // File is busy, increment the counter and go to the next loop
                        counter += 1;
                    },
                    false => {
                        // Name is free! Save it and exit the loop
                        final_path = candidate_name;
                        break;
                    }
                }
            }

            // Create a file with a unique filename
            match File::create(&final_path) {
                Ok(file) => {
                    // Write zero bytes to simulate an image. 32MB minimum size for the ext4 FS 
                    file.set_len(&args.size * 1024 * 1024)?;
                    println!("Successfully created file: {}", final_path);
                },
                Err(e) => eprintln!("Failed to create file {}: {}", final_path, e),
            }

        },
        false => {

            // create file in target directory
            match File::create(&new_path) {
                Ok(file) => {
                    // Write zero bytes to simulate an image. 32MB minimum size for the ext4 FS 
                    file.set_len(&args.size * 1024 * 1024)?;
                    println!("Successfully created file: {:?}", new_path);
                },
                Err(e) => eprintln!("Failed to create file {:?}: {}", new_path, e),
            }

        }

    }
 
    println!("Size: {} MB", &args.size);

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
    // Read current mount points from /proc/mounts
    let file = File::open("/proc/mounts").ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().flatten() {
        // Split the line by whitespace
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        // Mount entry format: [0] device, [1] mount point, [2] FS type
        if parts.len() >= 2 && parts[0] == device_path {
            return Some(PathBuf::from(parts[1]));
        }
    }
    None // If device is unmounted
}

pub fn check_mountpoint_status(path: &str) -> bool {
    let status = Command::new("moutpoint")
        .args(&["-q", path])
        .status();

    match status {
        Ok(exit_status) => exit_status.success(),
        Err(_) => false,
    }
}