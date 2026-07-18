mod parser;
mod core;


fn main() {

    println!("NOTE: This program requires root permissions (sudo) to execute the 'mount' command.");

    core::run();

}