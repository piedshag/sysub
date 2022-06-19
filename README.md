# Sysub

A proof of conecpt to intercept system calls and alter the register values using ptrace

## Usage

```
# ./test.sh
hello world
# ./ptrace -s "test"  -e ./test.sh -p "/home/target/x86_64-unknown-linux-gnu/release/output.log"
Intercepted read Fd: 0 Buf: 94103760409184 Size: 4096
test
Intercepted read Fd: 0 Buf: 94103760409184 Size: 4096
#
```

## References

https://www.cs.uaf.edu/2017/fall/cs301/lecture/09_11_registers.html
https://github.com/upenn-cis198/homework4/
https://github.com/skeeto/ptrace-examples/blob/master/xpledge.c
https://filippo.io/linux-syscall-table/
https://www.cs.fsu.edu/~langley/CNT5605/2017-Summer/assembly-example/assembly.html