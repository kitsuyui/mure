/// ## Description: A wrapper around std::process::Command to make it easier to use
/// ## Problem
///
/// In Rust, I want to express the result as a simple enum or struct as much as possible.
/// On the other hand, the result of a normal command is expressed by exit code and stdout, stderr.
/// The meaning of exit code and stderr varies depending on the command.
///
/// - Example 1. Most commands return 0 for success and anything else for failure.
/// - Example 2. The diff command for finding text differences returns 0 if there is no difference, 1 if there is a difference, and 2 if it fails.
/// - Example 3. git pull returns 0 if it is already up to date, but outputs "Already up to date" to stderr.
///
/// Since the meaning of exit code and stderr varies depending on the command, it cannot be expressed as a Rust Error.
/// If the return value is within the scope of the command, it should be expressed as an enum, and it is not appropriate to express it as a Rust Error.
/// The cases that should be expressed as a Rust Error are cases where the prerequisites of the command are not met.
/// For example,
///
/// - Example 1. The command is not installed
/// - Example 2. The environment variable required to execute the command is not set
/// - Example 3. An argument that the command cannot interpret is given
///
/// ## Solution provided by the wrapper
///
/// - This command wrapper expresses the result of the command as a RawCommandOutput struct.
/// - The type to be handled primarily in Rust is T.
/// - CommandOutput <T> holds both RawCommandOutput and the result of the command converted to T.
/// - Error holds RawCommandOutput.
use std::process::Output;

#[derive(Debug)]
pub enum Error {
    Raw(RawCommandOutput),
    FailedToExecute(std::io::Error),
}

#[derive(Debug)]
pub struct CommandOutput<T> {
    pub raw: RawCommandOutput,
    pub interpreted_to: T,
}

#[derive(Debug)]
pub struct RawCommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl RawCommandOutput {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

impl From<std::process::Output> for RawCommandOutput {
    fn from(output: Output) -> Self {
        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8(output.stdout).unwrap_or_default();
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();
        RawCommandOutput {
            status,
            stdout,
            stderr,
        }
    }
}

impl TryFrom<RawCommandOutput> for CommandOutput<()> {
    type Error = Error;

    fn try_from(raw: RawCommandOutput) -> Result<Self, Self::Error> {
        match raw.success() {
            true => raw.interpret_to(()),
            false => Err(Error::Raw(raw)),
        }
    }
}

impl TryFrom<Result<std::process::Output, std::io::Error>> for RawCommandOutput {
    type Error = Error;

    fn try_from(result: Result<std::process::Output, std::io::Error>) -> Result<Self, Self::Error> {
        match result {
            Ok(output) => Ok(RawCommandOutput::from(output)),
            Err(err) => Err(Error::FailedToExecute(err)),
        }
    }
}

impl RawCommandOutput {
    pub fn interpret_to<T>(self, item: T) -> Result<CommandOutput<T>, Error> {
        Ok(CommandOutput {
            raw: self,
            interpreted_to: item,
        })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Raw(raw) => write!(f, "{}", raw.stderr),
            Error::FailedToExecute(err) => write!(f, "Failed to execute command: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::process::Command;

    #[test]
    fn test_command_wrapper() {
        // success
        let output = Command::new("ls").arg("hello").output();
        let raw = RawCommandOutput::try_from(output);
        assert!(raw.is_ok());
        let raw = raw.unwrap();
        assert!(!raw.success());

        // fail: command not found
        let output = Command::new("nothing").arg("hello").output();
        let raw = RawCommandOutput::try_from(output);
        assert!(raw.is_err());
        let err = raw.err().unwrap();
        assert_eq!(
            err.to_string(),
            "Failed to execute command: No such file or directory (os error 2)"
        );
    }
}
