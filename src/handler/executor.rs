use std::{os::unix::process::CommandExt, process::Command, thread};

#[cfg_attr(test, mockall::automock)]
pub trait Executor {
    fn execute_in_background(&self, command: &str);
}

pub struct ThreadExecutor;

impl ThreadExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Executor for ThreadExecutor {
    fn execute_in_background(&self, command: &str) {
        let cmd = command
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        thread::Builder::new()
            .name("process_cmd".to_string())
            .spawn(move || {
                match Command::new(&cmd[0])
                    .env_remove("RUST_LOG")
                    .args(&cmd[1..])
                    .process_group(0)
                    .spawn()
                {
                    Ok(mut child) => {
                        let _ = child.wait();
                    }
                    Err(_) => {}
                }
            })
            .unwrap();
    }
}
