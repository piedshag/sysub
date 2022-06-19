use libc::{user_regs_struct, c_void};
use std::ffi::CString;
use std::os::unix::prelude::CommandExt;
use std::{process::Command, process::exit, process::Stdio};

use nix::{sys::ptrace};
use nix::sys::ptrace::AddressType;
use nix::unistd::{fork, ForkResult, Pid};
use nix::sys::wait::wait;
use nix::NixPath;

use std::fs::read_link;

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
    exe: String,

    /// Path to intercept
    #[clap(short, long)]
    path: String
}

fn run_child(exe: &str) {
    ptrace::traceme().unwrap();
    Command::new("cat").arg("output.log").exec();
    exit(0);
}

// newfstatat(int dfd, char *filename, struct stat *buf, int flag);
fn stat(reg: user_regs_struct, pid: Pid, intercept_path: &str) {
    let fd = reg.rdi;
    //let stat_object: libc::stat = unsafe{*(reg.rdx as *mut libc::stat)};

    let path = read_link(format!("/proc/{pid}/fd/{fd}", pid=pid, fd=fd)).unwrap();
    if path.to_str().unwrap() != intercept_path {
        return
    }

    println!("Stat {}",path.to_str().unwrap());
}

// read(int fildes, void *buf, size_t nbyte);
fn read(reg: user_regs_struct, pid: Pid, sub: &str) -> user_regs_struct {
    let mut new_regs = reg.clone();
    // Argument registers arg 1 = rd, arg 2 = rsi, arg 3 = rdx
    let fd = reg.rdi;
    let buf = reg.rsi;
    let size = reg.rdx;
    let bytes_read = reg.rax;

    println!("Read Fd: {:?} Buf: {:?} Size: {:?}, Read {} bytes", fd, buf, size, bytes_read);

    let string = CString::new(sub).unwrap();
    if bytes_read != 0 {
        new_regs.rax = string.len() as u64;
    }
    write_string(pid, buf as *mut c_void, string);
    new_regs
}

fn run_parent(child: Pid, sub: &str, intercept_path: &str) {
    // For each syscall it should loop twice
    // "syscall-enter-stop just prior to entering any system call" 
    // "syscall-exit-stop when the system call is finished, or if it is interrupted by a signal"
    // We only want to change registers on syscall-enter-stop
    let mut prev_rax = 0;
    wait().unwrap();
    ptrace::setoptions(child, ptrace::Options::PTRACE_O_TRACECLONE).unwrap();

    loop {
        // Check registers
        match ptrace::getregs(child) {
            Ok(x) => {
                // read
                if x.orig_rax == 0 && x.orig_rax == prev_rax { 
                    let fd = x.rdi;
                    let path = read_link(format!("/proc/{pid}/fd/{fd}", pid=child, fd=fd)).unwrap();
                    if path.to_str().unwrap() == intercept_path {
                        let new_regs = read(x, child, sub);
                        ptrace::setregs(child, new_regs);
                    }
                }

                // newfstatat
                //if x.orig_rax == 262 && x.orig_rax == prev_rax {
                //    stat(x, child, path);
                //}
                prev_rax = x.orig_rax;
            },
            Err(_) => break,
        };
        // Execute syscall
        ptrace::syscall(child, None).unwrap();
        wait().unwrap();
    }
}

fn main() {
    let args = Args::parse();

    match unsafe{fork()} {   
        Ok(ForkResult::Child) => {
            run_child(&args.exe);
        }   
        Ok(ForkResult::Parent {child}) => {
            run_parent(child, &args.sub, &args.path);
        } 
        Err(err) => {
            panic!("[main] fork() failed: {}", err);
        }
    };
}

fn write_string(pid: Pid, address: AddressType, string: CString) {
    let string_bytes = string.as_bytes();

    for i in 0..string.len() {
        let address = unsafe { address.offset(i as isize) };
        unsafe {
            ptrace::write(pid, address,  string_bytes[i] as *mut c_void ).unwrap();
        }
    }
}