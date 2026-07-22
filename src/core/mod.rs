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
        println!("Verbose mode enabled.");
    }

    // Dispatch CLI commands to scripts
    match cli.command {
        Commands::Diskmake(args) => {

            // Trim MB/GB from the variable
            let clean_digits: String = args.size.chars().filter(|c| c.is_ascii_digit()).collect();
            let parsed_mb = clean_digits.parse::<u64>().unwrap_or(1);

            // Parse size from GB to MB
            let size: u64 = match args.size {
                s if s.contains("MB") => parsed_mb,
                s if s.contains("GB") => parsed_mb * 1024,
                _ => parsed_mb, // Default to MB if the unit suffix is missing
            };

            // Build FileArgs from args fields
            let app_args = fs_utils::FileArgs {
                // target user for the disk
                user: args.user,
                // convert string to PathBuf
                image: args.image.clone(),
                // Image file size
                size: size,
            };

            match fs_utils::create_image_file(app_args) {
                Ok(()) => println!("Success! File created."),
                Err(e) => eprintln!("Error creating file: {}", e)
            }

            let losetup_create_device = Command::new("losetup")
                .args(&["-f", "--", &args.image])
                .status();

            match losetup_create_device {
                Ok(s) if s.success() => {
                    if let Ok(mut device) = fs_utils::get_device_name(&args.image) {
                        println!("Losetup create device: {}", device);

                        if args.encrypt {

                            println!("Enter password for {}: ", device);
                            let mut password = String::new();

                            std::io::stdin().read_line(&mut password).expect("Failed to read password");

                            crypt::CryptCommand::encrypt_loop(device.clone(), password.clone().trim().to_string()).execute();
                            crypt::CryptCommand::activate_loop(device.clone(), password.trim().to_string()).execute();

                            device = device.replace("/dev/", "/dev/mapper/crypt_");

                        };

                        fs_utils::LoopDevice::new(device.clone()).ensure_formatted(fs_utils::FileSystem::Ext4);

                        match fs::create_dir_all(&args.path) {
                            Ok(_) => fs_utils::DiskCommand::new_mount(device, args.path.clone()).execute(),
                            Err(e) => eprintln!("Failed create mount point {}: {}", args.path, e)
                        }

                    }
                }
                Ok(s) => eprintln!("losetup failed with exit code: {:?}", s.code()),
                Err(e) => eprintln!("Failed to execute command: {}", e),
            }

        }
        Commands::Mount(args) => {

            let losetup_create_device = Command::new("losetup")
                .args(&["-f", "--", &args.image])
                .status();

            match losetup_create_device {
                Ok(s) if s.success() => {
                    if let Ok(mut device) = fs_utils::get_device_name(&args.image) {
                        println!("losetup create device: {}", device);

                        if args.encrypt {

                            println!("Enter password for {}: ", device);
                            let mut password = String::new();

                            std::io::stdin().read_line(&mut password).expect("Failed to read password");

                            crypt::CryptCommand::activate_loop(device.clone(), password.trim().to_string()).execute();

                            device = device.replace("/dev/", "/dev/mapper/crypt_");
                        }

                        fs_utils::DiskCommand::new_mount(device, args.path).execute();

                    }
                }
                Ok(s) => eprintln!("losetup failed with exit code: {:?}", s.code()),
                Err(e) => eprintln!("Failed to execute command: {}", e),
            }

        }
        Commands::Umount(args) => {

            match fs_utils::get_loop_device(&args.path) {
                Ok(Some(device)) => {
                    println!("Device: {}", device);

                    fs_utils::DiskCommand::new_umount(args.path.clone()).execute();

                    if device.contains("mapper/crypt_") {
                        crypt::CryptCommand::deactivate_loop(device.clone()).execute()
                    }
                    
                    let detach_result = Command::new("losetup")
                        .args(&["-d", &device.replace("mapper/crypt_", "")])
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