#![allow(unused)]

pub const PROT_READ : u64 = 0x1;
pub const PROT_WRITE : u64 = 0x2;
pub const MAP_ANON : u64 = 0x20;
pub const MAP_PRIVATE: u64 = 0x02;
pub const MAP_SHARED: u64 = 0x01;
pub const CSIGNAL : u64 = 0x000000ff;	
pub const CLONE_VM : u64 = 0x00000100;	
pub const CLONE_FS : u64 = 0x00000200;	
pub const CLONE_FILES : u64 = 0x00000400;	
pub const CLONE_SIGHAND : u64 = 0x00000800;	
pub const CLONE_PTRACE : u64 = 0x00002000;	
pub const CLONE_VFORK : u64 = 0x00004000;	
pub const CLONE_PARENT : u64 = 0x00008000;	
pub const CLONE_THREAD : u64 = 0x00010000;	
pub const CLONE_NEWNS : u64 = 0x00020000;	
pub const CLONE_SYSVSEM : u64 = 0x00040000;	
pub const CLONE_SETTLS : u64 = 0x00080000;	
pub const CLONE_PARENT_SETTID : u64 = 0x00100000;	
pub const CLONE_CHILD_CLEARTID : u64 = 0x00200000;	
pub const CLONE_DETACHED : u64 = 0x00400000;	
pub const CLONE_UNTRACED : u64 = 0x00800000;	
pub const CLONE_CHILD_SETTID : u64 = 0x01000000;
pub const CLONE_NEWUTS : u64 = 0x04000000;	
pub const CLONE_NEWIPC : u64 = 0x08000000;	
pub const CLONE_NEWUSER : u64 = 0x10000000;	
pub const CLONE_NEWPID : u64 = 0x20000000;	
pub const CLONE_NEWNET : u64 = 0x40000000;	
pub const CLONE_IO : u64 = 0x80000000;	