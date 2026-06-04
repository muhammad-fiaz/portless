//! Child process management.

pub mod child;
pub mod kill;
pub mod logs;
pub mod spawn;

pub use child::{Child, ChildExit, ChildStatus};
pub use kill::{Pid, Signal, kill_process, pid_is_alive, signal_pid};
pub use logs::{
    MAX_LOG_FILES, MAX_LOG_SIZE, append_bytes, open_for_append, purge, rotate, rotate_if_needed,
    total_size,
};
pub use spawn::Spawner;
