mod brightness;
mod common;
pub mod complete;
pub mod completions;
use std::{env, ffi::OsString, time::Duration};

use dbus::blocking::Connection;
use structopt::StructOpt;

use gnome_randr::DisplayConfig;

pub mod modify;
pub mod query;

#[derive(StructOpt)]
enum Command {
    #[structopt(about = "List outputs, or inspect one output by connector such as eDP-1.")]
    Query(query::CommandOptions),
    #[structopt(
        about = "Change one output using query values, including preferred/refresh mode selection."
    )]
    Modify(modify::CommandOptions),
    #[structopt(about = "Print completions for bash, zsh, or fish.")]
    Completions(completions::CommandOptions),
}

#[derive(StructOpt)]
#[structopt(
    about = "A program to query information about and manipulate displays on Gnome with Wayland.",
    long_about = "A program to query information about and manipulate displays on Gnome with Wayland.\n\nDefault command is `query`. Run \"gnome-randr query\" first to list connector names such as \"eDP-1\" or \"HDMI-1\", plus each output's valid mode ids, scale factors, and current software brightness state."
)]
struct CLI {
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

pub(super) fn build_cli<'a, 'b>() -> structopt::clap::App<'a, 'b> {
    CLI::clap()
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args_os().collect::<Vec<OsString>>();
    if complete::try_handle(&args)? {
        return Ok(());
    }

    // Parse the CLI args. We do this first to short-circuit the dbus calls if there's an invalid arg.
    let args = CLI::from_iter(args);

    // See what we're executing
    let cmd = args.cmd.unwrap_or(Command::Query(query::CommandOptions {
        connector: None,
        summary: false,
        json: false,
    }));

    match cmd {
        Command::Query(opts) => {
            let conn = Connection::new_session()?;
            let proxy = conn.with_proxy(
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                Duration::from_millis(5000),
            );
            let config = DisplayConfig::get_current_state(&proxy)?;
            print!("{}", query::handle(&opts, &config, &proxy)?);
        }
        Command::Modify(opts) => {
            let conn = Connection::new_session()?;
            let proxy = conn.with_proxy(
                "org.gnome.Mutter.DisplayConfig",
                "/org/gnome/Mutter/DisplayConfig",
                Duration::from_millis(5000),
            );
            let config = DisplayConfig::get_current_state(&proxy)?;
            modify::handle(&opts, &config, &proxy)?;
        }
        Command::Completions(opts) => completions::handle(&opts),
    }

    Ok(())
}
