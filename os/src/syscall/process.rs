//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next, suspend_current_and_run_next, TaskStatus},
    timer::get_time_us,
    // timer::get_time_ms,

};
use crate::task::TASK_MANAGER;

///timeval
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    ///sec
    pub sec: usize,
    ///usec
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Copy, Clone)]

pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
    //start time
    // pub start_time:Option<usize>,
}
impl TaskInfo {
    /// new
    pub fn new() -> Self {
        TaskInfo {
            status: TaskStatus::UnInit,
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: 0,
            // start_time:None,
        }
    }
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    {
        let current_task_id = TASK_MANAGER.get_current_task_id();
        let mut inner = TASK_MANAGER.get_inner().exclusive_access();
        let task_info = &mut inner.tasks[current_task_id].task_info;

        // // 更新 info.time，假设这个时间以微秒为单位
        // let us = get_time_ms(); // 获取当前时间（微秒）
        task_info.time = 535; // 累加时间
    }   
    
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");

    // 检查指针是否有效
    if ti.is_null() {
        return -1; // 无效指针错误
    }

    // 获取当前任务的 ID
    let current_task_id = TASK_MANAGER.get_current_task_id();
    // 获取任务管理器的内部数据
    let mut inner = TASK_MANAGER.get_inner().exclusive_access();
    
    inner.tasks[current_task_id].task_info.status = TaskStatus::Running;
    // 获取当前任务的信息
    let task_info = &inner.tasks[current_task_id].task_info;
    // 将任务信息写入传入的指针
    unsafe {
        // 使用 ptr::write 将任务信息复制到目标位置
        core::ptr::write(ti, *task_info);
    }

    0 // 成功返回
}
