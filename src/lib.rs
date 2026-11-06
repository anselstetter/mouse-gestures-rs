mod cmd;
mod config;
mod device;
mod dispatcher;
mod handler;
mod info;
mod recognizer;
mod writer;

use crate::{
    cmd::{app_id, keys},
    config::window_provider::EnvWindowProvider,
    device::{
        input_manager::{DeviceType, InputManager},
        virtual_keyboard::VirtualKeyboard,
        virtual_mouse::VirtualMouse,
    },
    handler::executor::ThreadExecutor,
    writer::StdoutWriter,
};
use clap::{Parser, Subcommand};
use cmd::{start, trail};
use config::config::load_config;
use recognizer::angle_recognizer::AngleRecognizer;
use std::io::Result;

#[derive(Parser)]
#[command(name = "cli")]
#[command(about = "Hijack evdev to provide mouse gestures", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum InputType {
    Mouse,
    Keyboard,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start the recognizer", long_about = None)]
    Start,
    #[command(about = "Get mouse trail x,y coordinates", long_about = None)]
    Trail,
    #[command(about = "Get key and/or mouse combinations", long_about = None)]
    Keys {
        #[arg(long = "type", value_enum, num_args = 0..)]
        types: Vec<InputType>,
    },
    #[command(about = "Get an application id", long_about = None)]
    AppId,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let config = load_config()?;
    let window_provider = EnvWindowProvider::new()?;
    let recognizer = AngleRecognizer::new();
    let executor = ThreadExecutor::new();
    let writer = StdoutWriter::new();

    let virtual_mouse = VirtualMouse::new()?;
    let virtual_keyboard = VirtualKeyboard::new()?;

    let mouse = InputManager::new(DeviceType::Mouse)?;
    let keyboard = InputManager::new(DeviceType::Keyboard)?;

    match cli.command {
        Commands::Start => start::start(
            mouse,
            virtual_mouse,
            virtual_keyboard,
            recognizer,
            executor,
            config,
            window_provider,
        ),
        Commands::Trail => trail::start(mouse, virtual_mouse, writer),
        Commands::Keys { types } => keys::start(mouse, keyboard, virtual_mouse, writer, types),
        Commands::AppId => app_id::start(mouse, virtual_mouse, window_provider, writer),
    }
}
