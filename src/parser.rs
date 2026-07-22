use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;



const BANNER: &str = r#"
  ____   ____           ____    _____    ___    ____       _       ____   _____ 
 / ___| |  _ \         / ___|  |_   _|  / _ \  |  _ \     / \     / ___| | ____|
| |     | |_) |  ____  \___ \    | |   | | | | | |_) |   / _ \   | |  _  |  _|  
| |___  |  _ <  |____|  ___) |   | |   | |_| | |  _ <   / ___ \  | |_| | | |___ 
 \____| |_| \_\        |____/    |_|    \___/  |_| \_\ /_/   \_\  \____| |_____|
"#;

const EXAMPLES: &str = "Examples:\n  \
                        sudo crStorage create --name[-n] file_name  --size[-s] 32[MB/GB] --path[-p] /your/path\n  \
                        sudo crStorage delete --name[-n] file_name";

#[derive(Parser, Debug)]
#[command(
    name = "crStorage",
    author = "nav-uue", 
    version = "1.0.0", 
    about = "Simple encription tool", 
    long_about,
    before_help = BANNER,
    after_help(EXAMPLES)
)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {

    #[command(about = "Create a new image file, attach it as a loop device, and format with a file system")]
    Diskmake(DiskmakeArgs),

    #[command(about = "Attach and mount the image file to a loop device")]
    Mount(MountArgs),

    #[command(about = "Unmount the image and detach it from the loop device")]
    Umount(UmountArgs),

    // #[command(about = "Show information about existing images")]
    // Info(InfoArgs)
}

#[derive(Args, Debug)]
pub struct DiskmakeArgs {

    #[arg(short, long)]
    pub encrypt: bool,

    #[arg(short, long)]
    pub user: String,

    #[arg(short, long)]
    pub image: String,

    #[arg(short, long)]
    pub size: String,

    #[arg(short, long)]
    pub path: String

}

#[derive(Args, Debug)]
pub struct MountArgs {

    #[arg(short, long)]
    pub encrypt: bool,

    #[arg(short, long)]
    pub image: String,

    #[arg(short, long)]
    pub path: String

}

#[derive(Args, Debug)]
pub struct UmountArgs {

    #[arg(short, long)]
    pub path: String

}