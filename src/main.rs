use std::fs::{self, File};
use std::process::Command;
use std::path::Path;
use std::str;


const IMAGE_FILE: &str = "/tmp/test.image";
const MOUNT_POINT: &str = "/tmp/test";


fn main() {

    println!("--- Loop Device Mounting Demonstration ---");
    println!("NOTE: This program requires root permissions (sudo) to execute the 'mount' command.");
    
    // --- Setup ---
    if let Err(e) = setup_directories() {
        eprintln!("Error during setup: {}", e);
        return;
    }

    // --- 1. Create the image file ---
    match create_image_file() {
        Ok(()) => println!("Success! File created."),
        Err(e) => eprintln!("Error creating file: {}", e)
    }

    // --- 2. Mount the image as file system ---
    match mount_image() {
        Ok(()) => println!("\n✅ Successfully mounted {} as a loop device on {}", IMAGE_FILE, MOUNT_POINT),
        Err(e) => eprintln!("\n❌ Failed to mount image. Ensure you run the program with 'sudo'. Error: {}", e)
    }

    // Give the user time to inspect the mounted directory
    println!("\n--- Success! The mount point is active. ---");
    println!("Press Enter to clean up and exit...");
    let _ = std::io::stdin().read_line(&mut String::new());

    // --- 3. Umount and remove disk (only for testing) ---
    cleanup();

}

/// Create necessary directories.
fn setup_directories() -> Result<(), std::io::Error> {

    // Clean up any previous runs first
    if Path::new(IMAGE_FILE).exists() {
        fs::remove_file(IMAGE_FILE)?;
    }

    if Path::new(MOUNT_POINT).exists() {
        fs::remove_dir(MOUNT_POINT)?;
    }

    // Create the required mount point directory
    fs::create_dir_all(MOUNT_POINT)?;

    Ok(())

}

/// Create a small file to act the disk image
fn create_image_file() -> Result<(), std::io::Error> {

    println!("Create image file: {}", IMAGE_FILE);

    // create file in target directory
    let file = File::create(IMAGE_FILE)?;
 
    // Write zero bytes to simulate an image. 32MB minimum size for the ext4 FS 
    file.set_len(32 * 1024 * 1024)?;

    Ok(())

}

/// Execute `losetup` to attach the file to a loop device, and the `mount` to mount it.
fn mount_image() -> Result<(), String> {

    // 1. Use losetup to attach the image
    let losetup_create_image = Command::new("losetup")
        .args(&["-f", "--", IMAGE_FILE])
        .output()
        .map_err(|e| format!("Failed to execute losetup command: {}", e))?;

    /*

    Losetup keys:

    -f (or --find): Tells the system to scan for and use the first available, unused loop device (for example, /dev/loop0 or /dev/loop1).
    This prevents you from accidentally overwriting an active device mapping.

    -- (Double Dash): Signals the end of command options. Anything after -- is strictly treated as a positional argument (the filename).
    This is a safety measure in Linux. It protects the command from breaking if your file name happens to start with a dash (e.g., a file named -image.img).

    */

    if !losetup_create_image.status.success() {
        let stderr = String::from_utf8_lossy(&losetup_create_image.stderr);
        return Err(format!("losetup failed (exit code {}). Check permission/system status. Error output: {}", losetup_create_image.status.code().unwrap_or(-1), stderr));
    }

    // losetup successfull, we need to parse the output to find the device name
    let losetup_list_devices = Command::new("losetup")
        .arg("-a")
        .output()
        .map_err(|e| format!("Failed to execute losetup command: {}", e))?;

    let stdout_string = String::from_utf8_lossy(&losetup_list_devices.stdout);
    
    let loop_device = stdout_string
        .lines()
        .find(|line| line.contains(IMAGE_FILE))
        .map(|line| line.split(':').next().unwrap_or(line))
        .unwrap_or("N/A")
        .trim();

    if loop_device == "N/A" {
        return Err("Could not determine the loop device path from losetup output.".to_string());
    }

    let format = Command::new("mkfs.ext4")
        .arg(loop_device)
        .status()
        .unwrap();

    if format.success() {
        println!("Файлову систему ext4 успішно створено!")
    } else {
        eprintln!("Помилка під час форматування: {}", format)
    }

    println!("-> Attaching loop device: {}...", loop_device);

    // 2. Use mount to mount loop device
    let mount_result = Command::new("mount")
        .args(&[loop_device, MOUNT_POINT])
        .output()
        .map_err(|e| format!("Failed to execute mount command: {}", e));

    match mount_result {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("mount command failed (exit code {}). Error output: {}", output.status.code().unwrap_or(-1), stderr))
            }
        },
        Err(e) => Err(e)
    }
}

/// Unmounts and removes files/directories
fn cleanup() {

    println!("\n --- Starting cleanup --- ");

    // Umount first
    let umount_result = Command::new("umount")
        .args(&[MOUNT_POINT])
        .output()
        .map_err(|e| format!("Failed to execute umount: {}", e));

    match umount_result {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Warning could not unmount: {}. It might not have been mounted or permission were insufficient.", MOUNT_POINT);
            } else {
                println!("-> Successfully unmounted {}", MOUNT_POINT);
            }
        },
        Err(e) => eprintln!("Error during ummount attempt: {}", e)
    }

    // Parse losetup output and search loop device
    let losetup_list_devices = Command::new("losetup")
        .arg("-a")
        .output()
        .unwrap();

    let stdout_string = String::from_utf8_lossy(&losetup_list_devices.stdout);
    
    let loop_device = stdout_string
        .lines()
        .find(|line| line.contains(IMAGE_FILE))
        .map(|line| line.split(':').next().unwrap_or(line))
        .unwrap_or("N/A")
        .trim();

    // Detach loop device (This is the critical step otem missed)
    let detach_result = Command::new("losetup")
        .args(&["-d", loop_device])
        .output()
        .map_err(|e| format!("Failed to execute losetup detach: {}", e));

    match detach_result {
        Ok(output) => {
            if output.status.success() {
                println!("-> Successfully detach loop device.")
            } else {
                eprintln!("Warning: Could not detach loop device. May not been active.")
            }
        },
        Err(e) => eprintln!("Error during detach attempt: {}", e)
    }

    // Clean up files and directories
    let _ = fs::remove_dir(MOUNT_POINT);
    let _ = fs::remove_file(IMAGE_FILE);

    println!("-> Clenup complete. Temporary files removed.")

}
