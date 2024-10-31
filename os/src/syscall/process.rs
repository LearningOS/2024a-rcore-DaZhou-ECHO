//! Process management syscalls
#![allow(unused)]
use crate::{
    config::MAX_SYSCALL_NUM, mm::VirtAddr ,task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, 
        TASK_MANAGER,
    }, 
    timer::{get_time, get_time_ms},mm::KERNEL_SPACE,
    timer::get_time_us,
    mm:: {PhysPageNum, VirtPageNum}
};
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
// pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
//     trace!("kernel: sys_get_time");
//     if ts.is_null(){return -1;}
    
//     let a = ts as usize;
//     let va = VirtAddr::from(a);

//     if let Some(phys_addr) = va.virtaddr_convert_phyaddr() {
//         let us = get_time_us();
//         let kernel_ts = phys_addr.0 as *mut TimeVal;
//         unsafe {
//             *kernel_ts = TimeVal {
//                 sec: us / 1_000_000,
//                 usec: us % 1_000_000,
//             };
//         }
//         print!("wdaw-------{:?}",phys_addr);
//         0
//     } else {
//         -1
//     }
// }

use crate::mm::page_table::translated_byte_buffer;
use crate::task::current_user_token;
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let dst_vec = translated_byte_buffer(
        current_user_token(),
        ts as *const u8, core::mem::size_of::<TimeVal>()
    );
    let ref time_val = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
    };
    let src_ptr = time_val as *const TimeVal;
    for (idx, dst) in dst_vec.into_iter().enumerate() {
        let unit_len = dst.len();
        unsafe {
            dst.copy_from_slice(core::slice::from_raw_parts(
                src_ptr.wrapping_byte_add(idx * unit_len) as *const u8,
                unit_len)
            );
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
// pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
//     trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
//     if ti.is_null(){return  -1;}
//     if let Some(pa) = VirtAddr::from(ti as usize).virtaddr_convert_phyaddr(){
//         let task_info_ptr=pa.0 as *mut TaskInfo;
//         unsafe{
//             (*task_info_ptr).status = TaskStatus::Running;
//             (*task_info_ptr).time = get_time_ms() - TASK_MANAGER.get_start_time();
//             (*task_info_ptr).syscall_times = TASK_MANAGER.get_syscall_times();
//         }

//     }else{
//         return -1;
//     }
//     // -1
// }
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let dst_vec = translated_byte_buffer(
        current_user_token(),
        ti as *const u8, core::mem::size_of::<TaskInfo>()
    );
    let ref task_info = TaskInfo {
        status: TASK_MANAGER.get_statue(),
        syscall_times: TASK_MANAGER.get_syscall_times(),
        time: get_time_ms() - TASK_MANAGER.get_start_time(),
    };
    // println!("[kernel]: time {} syscall_time {}", task_info.time, task_info.syscall_times[super::SYSCALL_GET_TIME]);
    let src_ptr = task_info as *const TaskInfo;
    for (idx, dst) in dst_vec.into_iter().enumerate() {
        let unit_len = dst.len();
        unsafe {
            dst.copy_from_slice(core::slice::from_raw_parts(
                src_ptr.wrapping_byte_add(idx * unit_len) as *const u8,
                unit_len)
            );
        }
    }
    0
}
// // YOUR JOB: Implement mmap.
// pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
//     trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
//     -1
// }


use crate::config::{PAGE_SIZE, MAXVA};
use crate::task::{
    unmap_consecutive_area,
    create_new_map_area,
    get_current_task_page_table,
};
use crate::mm::{VPNRange, MapPermission};
/// mmap
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start >= MAXVA || start % PAGE_SIZE != 0 {
        return -1;
    }
    // avoid undefined situation
    let mut mlen = len;
    if start > MAXVA - len {
        mlen = MAXVA - start;
    }
    unmap_consecutive_area(start, mlen)
}
// // YOUR JOB: Implement munmap.
// pub fn sys_munmap(_start: usize, _len: usize) -> isize {
//     trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
//     -1
// }
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// port: page permission [2:0] X|W|R
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if start % PAGE_SIZE != 0 /* start need to be page aligned */ || 
        port & !0x7 != 0 /* other bits of port needs to be zero */ ||
        port & 0x7 ==0 /* No permission set, meaningless */ ||
        start >= MAXVA /* mapping range should be an legal address */ {
        return -1;
    }

    // check the range [start, start + len)
    // let start_va: VirtPageNum = VirtAddr::from(start).floor();
    // let end_va: VirtPageNum = VirtAddr::from(start + len).ceil();
    // let vpns = VPNRange::new(start_va, end_va);
    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();
    let vpns = VPNRange::new(start_vpn, end_vpn);
    for vpn in vpns {
       if let Some(pte) = get_current_task_page_table(vpn) {
            // we find a pte that has been mapped
            if pte.is_valid() {
                return -1;
            }
       }
    }
    // all ptes in range has pass the test
    create_new_map_area(
        // start_va.into(),
        // end_va.into(),
        start_vpn.into(),
        end_vpn.into(),
        MapPermission::from_bits_truncate((port << 1) as u8) | MapPermission::U
    );
    0
}