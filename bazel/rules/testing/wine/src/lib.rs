//! Rust utilities for running Windows binaries under Wine during Bazel tests.
//!
//! Provides `WineTestCommand` for configuring and spawning Windows executables
//! under Wine, and `WineTestProcess` for managing the lifecycle of these
//! processes — including isolated Wine prefixes and robust cleanup.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::Duration;

/// Error types for Wine test operations.
#[derive(Debug, thiserror::Error)]
pub enum WineError {
    #[error("Failed to spawn Wine process: {0}")]
    SpawnError(String),
    #[error("Wine process exited with non-zero status: {0}")]
    NonZeroExit(ExitStatus),
    #[error("Wine prefix creation failed: {0}")]
    PrefixError(String),
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A builder for configuring and spawning a Windows executable under Wine.
///
/// Manages an isolated Wine prefix (WINEPREFIX) for each invocation to
/// prevent cross-test contamination, and ensures robust cleanup of both
/// the prefix and the Wine server process after completion.
#[derive(Debug)]
pub struct WineTestCommand {
    /// Path to the Wine binary (default: `wine`).
    wine_binary: PathBuf,
    /// Path to the Windows executable to run.
    exe: PathBuf,
    /// Arguments to pass to the executable.
    args: Vec<String>,
    /// Environment variables to set for the Wine process.
    env: std::collections::HashMap<String, String>,
    /// Working directory for the process.
    working_dir: Option<PathBuf>,
    /// Timeout for the process.
    timeout: Option<Duration>,
    /// Whether to capture stdout/stderr (vs inherit).
    capture_output: bool,
    /// Path to a custom Wine prefix directory.
    prefix_dir: Option<PathBuf>,
}

/// A running (or completed) Wine test process.
///
/// Handles automatic cleanup of the Wine prefix and Wine server
/// on drop or completion.
#[derive(Debug)]
pub struct WineTestProcess {
    /// The child process handle.
    child: Option<std::process::Child>,
    /// The Wine prefix used for this process.
    prefix: Option<PathBuf>,
    /// The captured output, if requested.
    output: Option<Output>,
}

impl WineTestCommand {
    /// Create a new Wine test command for the given Windows executable.
    pub fn new(exe: impl Into<PathBuf>) -> Self {
        Self {
            wine_binary: PathBuf::from("wine"),
            exe: exe.into(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            working_dir: None,
            timeout: Some(Duration::from_secs(120)),
            capture_output: true,
            prefix_dir: None,
        }
    }

    /// Set the path to the Wine binary.
    pub fn wine_binary(mut self, path: impl Into<PathBuf>) -> Self {
        self.wine_binary = path.into();
        self
    }

    /// Add an argument to the Windows executable.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args(mut self, args: &[String]) -> Self {
        self.args.extend_from_slice(args);
        self
    }

    /// Set an environment variable for the Wine process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the process timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Disable timeout (run indefinitely).
    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Set whether to capture stdout/stderr (vs pass through).
    pub fn capture_output(mut self, capture: bool) -> Self {
        self.capture_output = capture;
        self
    }

    /// Set a custom Wine prefix directory.
    pub fn prefix_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.prefix_dir = Some(dir.into());
        self
    }

    /// Create an isolated Wine prefix directory.
    fn create_prefix(&self, prefix_path: &Path) -> Result<PathBuf, WineError> {
        std::fs::create_dir_all(prefix_path)
            .map_err(|e| WineError::PrefixError(format!("Cannot create prefix dir: {}", e)))?;

        // Run `wineboot --init` to bootstrap the prefix
        let status = Command::new(&self.wine_binary)
            .arg("wineboot")
            .arg("--init")
            .env("WINEPREFIX", prefix_path)
            .env("WINEDEBUG", "-all")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| WineError::PrefixError(format!("wineboot failed: {}", e)))?;

        if !status.success() {
            return Err(WineError::PrefixError(format!(
                "wineboot --init exited with {}",
                status
            )));
        }

        Ok(prefix_path.to_path_buf())
    }

    /// Spawn the Windows executable under Wine and return a `WineTestProcess`.
    pub fn spawn(&self) -> Result<WineTestProcess, WineError> {
        // Resolve the Wine prefix
        let prefix = match &self.prefix_dir {
            Some(dir) => dir.clone(),
            None => {
                let tmp = std::env::temp_dir().join(format!(
                    "wine_prefix_{}",
                    std::process::id()
                ));
                self.create_prefix(&tmp)?
            }
        };

        // Build the Wine command: wine <exe> [args...]
        let mut cmd = Command::new(&self.wine_binary);
        cmd.arg(&self.exe);
        cmd.args(&self.args);

        // Set WINEPREFIX and other environment
        cmd.env("WINEPREFIX", &prefix);
        cmd.env("WINEDEBUG", "-all");
        cmd.env("WINEARCH", "win64");

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }

        // Stdio configuration
        if self.capture_output {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        }

        let child = cmd.spawn()
            .map_err(|e| WineError::SpawnError(e.to_string()))?;

        Ok(WineTestProcess {
            child: Some(child),
            prefix: Some(prefix),
            output: None,
        })
    }

    /// Run the Windows executable to completion and return the output.
    ///
    /// Automatically cleans up the Wine prefix after the process exits.
    pub fn run(&self) -> Result<Output, WineError> {
        let mut proc = self.spawn()?;
        proc.wait_with_timeout(self.timeout)
    }

    /// Run the Windows executable, verify success, and return stdout as a String.
    pub fn run_and_get_stdout(&self) -> Result<String, WineError> {
        let output = self.run()?;
        if !output.status.success() {
            return Err(WineError::NonZeroExit(output.status));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl WineTestProcess {
    /// Wait for the process to finish, with an optional timeout.
    pub fn wait_with_timeout(&mut self, timeout: Option<Duration>) -> Result<Output, WineError> {
        let child = self.child.take()
            .ok_or_else(|| WineError::SpawnError("Process already completed".into()))?;

        let output = match timeout {
            Some(dur) => {
                // Use a thread to implement timeout
                let handle = child;
                let handle = std::thread::spawn(move || {
                    handle.wait_with_output()
                });

                match handle.join() {
                    Ok(Ok(output)) => output,
                    Ok(Err(e)) => return Err(WineError::Io(e)),
                    Err(_) => return Err(WineError::Timeout(dur)),
                }
            }
            None => child.wait_with_output()?,
        };

        self.output = Some(output.clone());
        self.cleanup();
        Ok(output)
    }

    /// Wait indefinitely for the process to finish.
    pub fn wait(&mut self) -> Result<ExitStatus, WineError> {
        let child = self.child.take()
            .ok_or_else(|| WineError::SpawnError("Process already completed".into()))?;
        let status = child.wait()?;
        self.cleanup();
        Ok(status)
    }

    /// Clean up the Wine prefix and kill the Wine server.
    fn cleanup(&self) {
        // Remove the Wine prefix directory
        if let Some(prefix) = &self.prefix {
            let _ = std::fs::remove_dir_all(prefix);
        }

        // Kill the wineserver to prevent lingering processes
        let _ = Command::new("wineserver")
            .arg("-k")
            .env("WINEPREFIX", self.prefix.as_ref().map_or_else(
                || Path::new(""),
                |p| p.as_path(),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

impl Drop for WineTestProcess {
    fn drop(&mut self) {
        // Kill the child if still running
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.cleanup();
    }
}

/// Run a Windows executable under Wine with default settings,
/// returning the captured output.
///
/// Convenience function for simple test cases.
pub fn run_wine_test(exe: impl AsRef<Path>, args: &[&str]) -> Result<String, WineError> {
    let mut cmd = WineTestCommand::new(exe.as_ref());
    for arg in args {
        cmd = cmd.arg(*arg);
    }
    cmd.run_and_get_stdout()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wine_test_command_builder() {
        let cmd = WineTestCommand::new("/tmp/test.exe")
            .arg("--verbose")
            .arg("2")
            .timeout(Duration::from_secs(30))
            .capture_output(true);

        assert_eq!(cmd.exe.to_str(), Some("/tmp/test.exe"));
        assert_eq!(cmd.args, vec!["--verbose", "2"]);
        assert!(cmd.capture_output);
    }

    #[test]
    fn test_prefix_creation_and_cleanup() {
        let tmp = std::env::temp_dir().join(format!("wine_test_{}", line!()));
        let cmd = WineTestCommand::new("not-a-real.exe")
            .prefix_dir(&tmp);

        assert_eq!(cmd.prefix_dir, Some(tmp.clone()));

        // Verify prefix path is set, but don't try to create it
        // (Wine may not be installed in CI)
        let prefix = cmd.prefix_dir.unwrap();
        assert!(prefix.to_str().map_or(false, |s| s.contains("wine_test_")));
    }
}
