use clap::Parser;
use bevy::prelude::Resource;

#[derive(Parser, Resource, Debug, Clone)]
pub struct Args {
    #[clap(long)]
    pub synctest: bool,
    #[clap(long, default_value = "2")]
    pub input_delay: usize,
}
