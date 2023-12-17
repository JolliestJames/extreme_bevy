use clap::Parser;
use bevy::prelude::Resource;

#[derive(Parser, Resource, Debug, Clone)]
pub struct Args {
    #[clap(long)]
    pub synctest: bool,
}
