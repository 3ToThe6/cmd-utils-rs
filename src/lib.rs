#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::io::Write;
use std::process::Command;

use anyhow::Context;
use termcolor::WriteColor;

pub trait CommandExt {
    fn exec(&mut self) -> anyhow::Result<()>;
    fn exec_args<I, S>(&mut self, args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;
    fn exec_stdout_string(&mut self) -> anyhow::Result<String>;
}

impl CommandExt for Command {
    fn exec(&mut self) -> anyhow::Result<()> {
        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream};

        let mut stderr = StandardStream::stderr(ColorChoice::Auto);

        let current_dir =
            std::env::current_dir().with_context(|| "Failed to get current working directory")?;
        let current_dir_color_spec = {
            let mut spec = ColorSpec::new();
            spec.set_bg(Some(Color::Cyan));
            spec.set_fg(Some(Color::Black));
            spec
        };
        stderr.with_color(&current_dir_color_spec, |s| {
            write!(s, "{}", current_dir.display()).unwrap()
        });
        writeln!(stderr, " {:?}", self).unwrap();

        let cmd_success = self.status().with_context(|| "Failed to execute command")?.success();

        let eo_color_spec = {
            let mut spec = ColorSpec::new();
            if cmd_success {
                spec.set_bg(Some(Color::Green));
            } else {
                spec.set_bg(Some(Color::Red));
            }
            spec.set_fg(Some(Color::Black));
            spec
        };
        stderr.with_color(&eo_color_spec, |s| write!(s, " END OUTPUT ").unwrap());
        writeln!(stderr).unwrap();
        Ok(())
    }
    fn exec_args<I, S>(&mut self, args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args(args);
        self.exec()
    }

    fn exec_stdout_string(&mut self) -> anyhow::Result<String> {
        use std::process::{Output, Stdio};
        let Output { status, stdout, stderr } = self
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute command ({})", cmd_info(self)))?;
        if !status.success() {
            anyhow::bail!(
                "Process did not exit successfully: ({})",
                cmd_info_with_output(self, &stdout, &stderr),
            );
        }
        let stdout = String::from_utf8(stdout).map_err(|e| {
            let context = format!(
                "Process stdout is not UTF-8: {}",
                cmd_info_with_output(self, e.as_bytes(), &stderr),
            );
            anyhow::Error::new(e).context(context)
        })?;
        Ok(stdout)
    }
}

pub fn cmd(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}

pub trait TermColorStandardStreamExt {
    fn with_color<F, T>(&mut self, spec: &termcolor::ColorSpec, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T;
}

impl TermColorStandardStreamExt for termcolor::StandardStream {
    fn with_color<F, T>(&mut self, spec: &termcolor::ColorSpec, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.set_color(spec).unwrap();
        let v = f(self);
        self.reset().unwrap();
        v
    }
}

pub fn stdout_with_color<F, T>(spec: &termcolor::ColorSpec, f: F) -> T
where
    F: FnOnce(&mut termcolor::StandardStream) -> T,
{
    termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto).with_color(spec, f)
}

pub fn stderr_with_color<F, T>(spec: &termcolor::ColorSpec, f: F) -> T
where
    F: FnOnce(&mut termcolor::StandardStream) -> T,
{
    termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto).with_color(spec, f)
}

fn cmd_info(cmd: &Command) -> String {
    format!(
        "program = {:?}, args = {:?}, envs = {:?}, current_dir = {:?}",
        cmd.get_program(),
        cmd.get_args(),
        cmd.get_envs(),
        cmd.get_current_dir(),
    )
}

fn cmd_info_with_output(cmd: &Command, stdout: &[u8], stderr: &[u8]) -> String {
    format!(
        "{}, stdout = {}, stderr = {}",
        cmd_info(cmd),
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr),
    )
}
