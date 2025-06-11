use libc_core::{time::ITimerVal, types::TimeVal};

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessTimer {
    pub timer: ITimerVal,
    pub next: TimeVal,
    pub last: TimeVal,
}
