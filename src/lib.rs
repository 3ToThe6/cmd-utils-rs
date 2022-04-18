#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fmt::Display;
use std::io::Write;
use std::process::Command;

use anyhow::Context;
use termcolor::WriteColor;

pub trait CommandExt {
    fn description(&self) -> CommandDescription<'_>;

    fn args_<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;

    fn exec(&mut self) -> anyhow::Result<()>;
    fn exec_args<I, S>(&mut self, args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>;

    fn exec_stdout_string(&mut self) -> anyhow::Result<String>;
}

pub struct CommandDescription<'a> {
    cmd: &'a Command,
}

impl CommandExt for Command {
    fn description(&self) -> CommandDescription<'_> {
        CommandDescription { cmd: self }
    }

    fn args_<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut self_ = self;
        self_.args(args);
        self_
    }

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
            .with_context(|| format!("Failed to execute command ({})", self.description()))?;
        if !status.success() {
            anyhow::bail!(
                "Process did not exit successfully ({})",
                cmd_info_with_output(self, &stdout, &stderr),
            );
        }
        let stdout = String::from_utf8(stdout).map_err(|e| {
            let context = format!(
                "Process stdout is not UTF-8 ({})",
                cmd_info_with_output(self, e.as_bytes(), &stderr),
            );
            anyhow::Error::new(e).context(context)
        })?;
        Ok(stdout)
    }
}

impl Display for CommandDescription<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "program = {:?}, args = {:?}, envs = {:?}, current_dir = {:?}",
            self.cmd.get_program(),
            self.cmd.get_args(),
            self.cmd.get_envs(),
            self.cmd.get_current_dir(),
        )
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
    let color_choice = match atty::is(atty::Stream::Stdout) {
        true => termcolor::ColorChoice::Auto,
        false => termcolor::ColorChoice::Never,
    };
    termcolor::StandardStream::stdout(color_choice).with_color(spec, f)
}

pub fn stderr_with_color<F, T>(spec: &termcolor::ColorSpec, f: F) -> T
where
    F: FnOnce(&mut termcolor::StandardStream) -> T,
{
    let color_choice = match atty::is(atty::Stream::Stderr) {
        true => termcolor::ColorChoice::Auto,
        false => termcolor::ColorChoice::Never,
    };
    termcolor::StandardStream::stderr(color_choice).with_color(spec, f)
}

pub fn cmd_info_with_output(cmd: &Command, stdout: &[u8], stderr: &[u8]) -> String {
    format!(
        "{}, stdout = {:?}, stderr = {:?}",
        cmd.description(),
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr),
    )
}
