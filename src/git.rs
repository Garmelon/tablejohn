//! Execute git binary commands.

use std::{
    error, fmt, io,
    path::Path,
    process::{Command, Output},
};

use log::trace;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Command(Command, Output),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Command(command, output) => {
                write!(f, "Command exited with {}", output.status)?;
                write!(f, "COMMAND: {command:?}")?;
                if !output.stdout.is_empty() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    write!(f, "STDOUT:\n{}", stdout.trim_end())?;
                }
                if !output.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    write!(f, "STDERR:\n{}", stderr.trim_end())?;
                }
                Ok(())
            }
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn run(mut command: Command) -> Result<Output, Error> {
    let output = command.output()?;
    if output.status.success() {
        trace!("Command exited with {}", output.status);
        trace!("COMMAND: {command:?}");
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            trace!("STDOUT:\n{}", stdout.trim_end());
        }
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            trace!("STDERR:\n{}", stderr.trim_end());
        }
        Ok(output)
    } else {
        Err(Error::Command(command, output))
    }
}

pub fn fetch(path: &Path, url: &str, refspecs: &[String]) -> Result<(), Error> {
    let mut command = Command::new("git");
    command
        .arg("fetch")
        .arg("-C")
        .arg(path)
        .arg("--prune")
        .arg("--")
        .arg(url);
    for refspec in refspecs {
        command.arg(refspec);
    }
    run(command)?;
    Ok(())
}
