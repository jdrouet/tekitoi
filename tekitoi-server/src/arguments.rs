use clap::Parser;
use std::path::PathBuf;
use tekitoi_server::settings::Settings;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Arguments {
    /// Path to a configuration file
    #[clap(short, long)]
    config: Option<PathBuf>,
}

impl Arguments {
    pub fn build() -> Self {
        Self::parse()
    }

    pub fn settings(&self) -> Settings {
        Settings::build(&self.config)
    }
}
