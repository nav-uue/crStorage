use std::fs::{self, File};
use std::process::Command;
use std::path::{Path, PathBuf};
use clap::Parser;

use crate::parser::{Cli, Commands};
mod fs_utils;
mod crypt;


pub fn run() {

    let cli = Cli::parse();
    
    // Check global flag
    if cli.verbose {
        println!("Режим подробного вывода включен.");
    }

    // Dispatch CLI commands to scripts
    match cli.command {
        Commands::Create(args) => {

            // Trim MB/GB from the variable
            let clean_digits: String = args.size.chars().filter(|c| c.is_ascii_digit()).collect();

            // Build FileArgs from args fields
            let app_args = fs_utils::FileArgs {

                // convert string to PathBuf
                path: PathBuf::from(args.path),

                // Pass string as-is
                name: args.name,

                // convert string to u64
                size: clean_digits.parse::<u64>().unwrap_or(1),

            };

            match fs_utils::create_image_file(app_args) {
                Ok(()) => println!("Success! File created."),
                Err(e) => eprintln!("Error creating file: {}", e)
            }

        }
        Commands::Delete(args) => {
            let path = format!("{}",args.path);
            println!("Delete file: {}", &path);
            if Path::new(&path).exists() {
                fs::remove_file(&path).unwrap_or_else(|err| {
                    eprintln!("Error: file not exists or cannot be removed! Details: {}", err);
                });
            }
        }
        Commands::Mount(args) => {

            let losetup_create_image = Command::new("losetup")
                .args(&["-f", "--", &args.device])
                .status();

            match losetup_create_image {
                Ok(s) if s.success() => {
                    if let Ok(device) = fs_utils::get_device_name(&args.device) {
                        println!("losetup create device: {}", device);

                        let f = crypt::CryptCommand::encrypt_loop(device.clone(), "test".to_string());
                        f.execute();

                        let c = crypt::CryptCommand::activate_loop(device.clone(), "test".to_string());
                        c.execute();

                        let encrypt_device = device.replace("/dev/", "/dev/mapper/crypt_");

                        let fsystem = fs_utils::LoopDevice::new(encrypt_device.clone());
                        fsystem.ensure_formatted(fs_utils::FileSystem::Ext4);

                        let mount_cmd = fs_utils::DiskCommand::new_mount(encrypt_device, args.path);
                        mount_cmd.execute()
                    }
                }
                Ok(s) => eprintln!("losetup failed with exit code: {:?}", s.code()),
                Err(e) => eprintln!("Failed to execute command: {}", e),
            }

        }
        Commands::Umount(args) => {

            match fs_utils::get_loop_device(&args.path) {
                Ok(Some(device)) => {
                    println!("Device: {}", &device);

                    let umount_cmd = fs_utils::DiskCommand::new_umount(args.path.clone());
                    umount_cmd.execute();
                    
                    let detach_result = Command::new("losetup")
                        .args(&["-d", &device])
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

                },
                Ok(None) => println!("Mount point not found"),
                Err(e) => eprintln!("Read error /proc/mounts: {}", e),
            }

        }
    }

}