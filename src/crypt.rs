use std::process::Command;
use std::path::Path;

use libcryptsetup_rs::{
    c_uint,
    consts::{
        flags::{CryptVolumeKey, CryptActivate, CryptDeactivate},
        vals::EncryptionFormat
    },
    CryptInit, LibcryptErr, TokenInput,
};

use crate::fs_utils::get_mount_point;


#[derive(Debug)]
pub enum CryptCommand {
    ActivateLoop {
        device_path: String,
        password: String
    },
    DeactivateLoop {
        device_path: String
    },
    EncryptLoop {
        device_path: String,
        password: String
    }
}


impl CryptCommand {

    pub fn activate_loop(device_path: String, password: String) -> Self {
        Self::ActivateLoop {
            device_path,
            password
        }
    }

    pub fn deactivate_loop(device_path: String) -> Self {
        Self::DeactivateLoop {
            device_path
        }
    }

    pub fn encrypt_loop(device_path: String, password: String) -> Self {
        Self::EncryptLoop {
            device_path,
            password
        }
    }

    pub fn execute(&self) {

        match self {
            CryptCommand::ActivateLoop { device_path, password } => {

                let mapper_name = &device_path.replace("/dev/", "crypt_");
                
                let mut device = match CryptInit::init(Path::new(&device_path)) {
                    Ok(device) => device,
                    Err(e) => {
                        eprintln!("Failed to initialize device activate {}: {}", device_path, e);
                        return;
                    }
                };

                match device.context_handle().load::<()>(Some(EncryptionFormat::Luks2), None) {
                    Ok(_) => {
                        match device.activate_handle().activate_by_passphrase(Some(&mapper_name), None, &password.as_bytes(), CryptActivate::empty()) {
                            Ok(_) => println!("Device {} successfully open.", &device_path.replace("/dev/", "/dev/mapper/crypt_")),
                            Err(e) => eprintln!("Failed to activate device: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("Failed to activate device: {:?}", e)
                }

            }
            CryptCommand::DeactivateLoop { device_path } => {

                if let Some(mount_path) = get_mount_point(&device_path.replace("/dev/", "/dev/mapper/crypt_")) {

                    let status = std::process::Command::new("umount")
                        .arg("-l")
                        .arg(mount_path)
                        .status()
                        .unwrap();
                    if status.success() {
                        println!("Successfully umount")
                    }
                } else {
                    println!("Device is not mounted!");
                }


                let mapper_name = device_path.replace("/dev/", "crypt_");
                println!("mapper_name: {}", &mapper_name);

                let mut cd = match CryptInit::init(Path::new(&device_path)) {
                    Ok(device) => device,
                    Err(e) => {
                        eprintln!("Failed to initialize device deactivation {}: {}", device_path, e);
                        return;
                    }
                };

                match cd.context_handle().load::<()>(Some(EncryptionFormat::Luks2), None) {
                    Ok(_) => {
                        match cd.activate_handle().deactivate(&mapper_name, CryptDeactivate::empty()) {
                            Ok(_) => println!("Device /dev/mapper/{} successfully closed and locked", mapper_name),
                            Err(e) => eprintln!("Failed to deactivate device: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("Failed to deactivate device: {:?}", e),
                };
                
            }

            CryptCommand::EncryptLoop { device_path, password } => {

                match CryptInit::init(Path::new(&device_path)) {
                    Ok(mut device) => {
                        
                        match device.context_handle().format::<()>(
                            EncryptionFormat::Luks2,
                            ("aes", "xts-plain"),
                            None,
                            libcryptsetup_rs::Either::Right(256 / 8),
                            None,
                        ) {
                            Ok(_) => {
                                println!("Device formatted. Fetching key handle...");

                                match device.keyslot_handle().add_by_key(None, None, password.as_bytes(), CryptVolumeKey::empty()) {
                                    Ok(_) => {
                                        println!("Success: Device {} encrypted with LUKS2.", device_path);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to add passphrase: {:?}", e);
                                    }
                                }
                                   
                            }
                            Err(e) => {
                                eprintln!("Formatting error (check root privileges): {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed initialize device: {:?}", e);
                    }
                }
            }
        }

    }

}