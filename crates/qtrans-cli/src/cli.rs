use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "qtrans",
    version,
    about = "Minecraft modpack quest translator (FTB Quests + KubeJS)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Extract {
        zip: PathBuf,
        #[arg(long, default_value = "work")]
        work: PathBuf,
        #[arg(long, default_value = "zh_tw")]
        target: String,
    },
    Scan {
        #[arg(long)]
        project: PathBuf,
    },
    Translate {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        batch_size: Option<usize>,
    },
    Apply {
        #[arg(long)]
        project: PathBuf,
    },
    Build {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        project: PathBuf,
    },
    Run {
        zip: PathBuf,
        #[arg(long, default_value = "work")]
        work: PathBuf,
        #[arg(long, default_value = "zh_tw")]
        target: String,
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        service: String,
        #[arg(long, default_value = "google_free")]
        provider: String,
        #[arg(long, default_value = "")]
        model: String,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        batch_size: Option<usize>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        skip_translate: bool,
    },
}
