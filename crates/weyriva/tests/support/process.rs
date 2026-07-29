#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::Mutex;

use weyriva::process::{CommandSpec, ExecRunner, ProcessOutput, ProcessRunner};
use weyriva::{Error, Result};

#[derive(Default)]
pub struct RecordingProcess {
    commands: Mutex<Vec<CommandSpec>>,
    outputs: Mutex<VecDeque<Result<ProcessOutput>>>,
}

impl RecordingProcess {
    pub fn push(&self, code: i32, stdout: &str, stderr: &str) {
        self.outputs
            .lock()
            .expect("output lock should remain available")
            .push_back(Ok(ProcessOutput {
                code,
                stdout: stdout.to_owned(),
                stderr: stderr.to_owned(),
            }));
    }

    pub fn fail(&self, code: &str, message: &str) {
        self.outputs
            .lock()
            .expect("output lock should remain available")
            .push_back(Err(Error::new(code, message)));
    }

    pub fn commands(&self) -> Vec<CommandSpec> {
        self.commands
            .lock()
            .expect("command lock should remain available")
            .clone()
    }
}

impl ProcessRunner for RecordingProcess {
    fn run(&self, command: &CommandSpec) -> Result<ProcessOutput> {
        self.commands
            .lock()
            .expect("command lock should remain available")
            .push(command.clone());
        self.outputs
            .lock()
            .expect("output lock should remain available")
            .pop_front()
            .unwrap_or_else(|| {
                Ok(ProcessOutput {
                    code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            })
    }
}

impl ExecRunner for RecordingProcess {
    fn exec(&self, command: &CommandSpec) -> Result<()> {
        self.commands
            .lock()
            .expect("command lock should remain available")
            .push(command.clone());
        Err(Error::new("exec_intercepted", "test intercepted exec"))
    }
}
