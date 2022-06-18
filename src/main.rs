use libc::{user_regs_struct, c_void};
use std::ffi::CString;
use std::os::unix::prelude::CommandExt;
use std::{process::Command, process::exit, process::Stdio};

use nix::{sys::ptrace};
use nix::sys::ptrace::AddressType;
use nix::unistd::{fork, ForkResult, Pid};
use nix::sys::wait::wait;
use nix::NixPath;

use clap::Parser;

/// Intercept write syscalls and swap out the buf
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// String to substitute in
    #[clap(short, long)]
    sub: String,

    /// Program to trace
    #[clap(short, long)]
    exe: String
}

fn run_child(exe: &str) {
    ptrace::traceme().unwrap();
    Command::new(exe).stdout(Stdio::null()).exec();
    exit(0);
}

fn read(reg: user_regs_struct) {
    let fd = reg.rdi;
    let buf = reg.rsi;
    let size = reg.rdx;

    println!("Read Fd: {:?} Buf: {:?} Size: {:?}", fd, buf, size);
}

fn write(reg: user_regs_struct, pid: Pid, sub: &str) -> user_regs_struct {
    let mut new_regs = reg.clone();
    // Argument registers arg 1 = rd, arg 2 = rsi, arg 3 = rdx
    let fd = reg.rdi;
    let buf = reg.rsi;
    let size = reg.rdx;

    println!("Write Fd: {:?} Buf: {:?} Size: {:?}", fd, buf, size);

    let string = CString::new(sub).unwrap();
    new_regs.rdx = string.len() as u64;

    write_string(pid, buf as *mut c_void, string);

    return new_regs
}

fn run_parent(child: Pid, sub: &str) {
    // For each syscall it should loop twice
    // "syscall-enter-stop just prior to entering any system call" 
    // "syscall-exit-stop when the system call is finished, or if it is interrupted by a signal"
    // TODO fix bug where write syscalls are repeated
    loop {
        // Wait for child to finish instruction
        wait().unwrap();
        // Check registers
        match ptrace::getregs(child) {
            Ok(x) => {
                if x.orig_rax == 1 { // write
                    let new_regs = write(x, child, sub);
                    ptrace::setregs(child, new_regs).unwrap();
                }
            },
            Err(_) => break,
        };
        // Execute syscall
        ptrace::syscall(child, None).unwrap();
    }
}

fn main() {
    let args = Args::parse();

    match unsafe{fork()} {   
        Ok(ForkResult::Child) => {
            run_child(&args.exe);
        }   
        Ok(ForkResult::Parent {child}) => {
            run_parent(child, &args.sub);
        } 
        Err(err) => {
            panic!("[main] fork() failed: {}", err);
        }
    };
}

fn write_string(pid: Pid, address: AddressType, string: CString) {
    let mut count = 0;
    let string_bytes = string.as_bytes();

    for i in 0..string.len() {
        let address = unsafe { address.offset(count) };
        unsafe {
            ptrace::write(pid, address,  string_bytes[i] as *mut c_void ).unwrap();
        }
        count += 1;
    }
}