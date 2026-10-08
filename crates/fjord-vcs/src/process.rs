//! Running command-line tools (git, gh, az) from a desktop app.
//!
//! On Windows two things differ: a GUI app that starts a console program flashes
//! a console window unless told not to, and the Azure CLI is a batch script
//! (`az.cmd`) that `CreateProcess` won't find without going through `cmd`.

use std::process::Command;

/// A `Command` for a CLI tool that behaves the same on every platform.
pub(crate) fn tool(program: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = if program == "az" {
            let mut c = Command::new("cmd");
            c.args(["/D", "/C", "az"]);
            c
        } else {
            Command::new(program)
        };
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}
