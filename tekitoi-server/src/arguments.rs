use clap::Parser;
use std::path::PathBuf;
use tekitoi::settings::Settings;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Arguments {
    /// Path to a configuration file
    #[clap(short, long)]
    config: Option<PathBuf>,
}

impl Arguments {
    pub(crate) fn build() -> Self {
        Self::parse()
    }

    pub(crate) fn settings(&self) -> Settings {
        Settings::build(self.config.clone())
    }
}
