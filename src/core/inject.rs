use std::{
    fs, mem::{self, MaybeUninit}, os::windows::ffi::OsStrExt, path::Path
};

use windows::{core::{s, PWSTR}, Win32::{Foundation::{CloseHandle, HANDLE, LUID}, System::{Diagnostics::Debug::WriteProcessMemory, LibraryLoader::{GetModuleHandleW, GetProcAddress}, Memory::{self, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE}, Threading::{CreateRemoteThread, WaitForSingleObject, CREATE_NEW_CONSOLE, INFINITE, PROCESS_INFORMATION, STARTUPINFOW}}}};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, SE_DEBUG_NAME, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::{
    core::w,
    Win32::System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{
            self, CreateProcessW, GetCurrentProcess, OpenProcessToken, PROCESS_ALL_ACCESS,
        },
    },
};

// https://stackoverflow.com/questions/865152/how-can-i-get-a-process-handle-by-its-name-in-c
// https://github.com/3gstudent/Inject-dll-by-APC/blob/master/CreateRemoteThread.cpp
fn enable_debug_priv() {
    unsafe {
        let mut h_token: MaybeUninit<HANDLE> = MaybeUninit::uninit();
        let mut luid: MaybeUninit<LUID> = MaybeUninit::zeroed();
        let tkp: MaybeUninit<TOKEN_PRIVILEGES> = MaybeUninit::zeroed();
        match OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            h_token.as_mut_ptr(),
        ) {
            Ok(_) => {
                LookupPrivilegeValueW(
                    None,
                    SE_DEBUG_NAME,
                    luid.as_mut_ptr()
                ).unwrap();

                tkp.assume_init().PrivilegeCount = 1;
                tkp.assume_init().Privileges[0].Luid = luid.assume_init();
                tkp.assume_init().Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

                AdjustTokenPrivileges(
                    h_token.assume_init(),
                    false,
                    Some(tkp.as_ptr()),
                    mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                    None,
                    None,
                ).unwrap();
        
                CloseHandle(h_token.assume_init()).unwrap();        
            },
            Err(e) => println!("{}", e)
        }
    }
}

fn find_process(process_name: &str) -> Option<HANDLE> {
    unsafe {
        let process_name = process_name.to_lowercase();
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();

        if Process32FirstW(snapshot, &mut entry as _).is_ok() {
            loop {
                let proc_name: String = String::from_utf16_lossy(entry.szExeFile.as_slice());
                // This is case sensitive
                if proc_name.to_lowercase().starts_with(&process_name) {
                    let h_process = Threading::OpenProcess(
                        PROCESS_ALL_ACCESS,
                        false,
                        entry.th32ProcessID,
                    ).unwrap();
                    return Some(h_process);
                }
                if Process32NextW(snapshot, &mut entry as _).is_err() {
                    break;
                }
            }
        }
        None
    }
}

pub fn inject_payload(process: HANDLE, module_path: &str) {
    unsafe {
        let path = fs::canonicalize(Path::new(module_path)).unwrap();
        let mut module_path_buf = path.as_os_str().encode_wide().collect::<Vec<u16>>();
        // Add null terminator
        module_path_buf.push(0);
        // # of bytes
        let len = module_path_buf.len() * 2;

        let p_thread_data = Memory::VirtualAllocEx(
            process,
            None,
            len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE
        );

        WriteProcessMemory(
            process,
            p_thread_data,
            module_path_buf.as_ptr() as _,
            len,
            None
        ).unwrap();

        let kernel_module = GetModuleHandleW(w!("kernel32")).unwrap();
        let load_library_addr = GetProcAddress(kernel_module, s!("LoadLibraryW"));
        let h_thread = CreateRemoteThread(
            process,
            None,
            0,
            mem::transmute(load_library_addr),
            Some(p_thread_data),
            0,
            None
        );
        match h_thread {
            Ok(v) => {
                WaitForSingleObject(v, INFINITE);
            },
            Err(_) => {
                CloseHandle(process).unwrap();
                println!("Failed to create remote thread");
            },
        }
    }
}

pub fn hijack_process(process_name: &str, module_path: &str) {
    enable_debug_priv();
    match find_process(process_name) {
        Some(proc) => {
            inject_payload(proc, module_path);
        },
        None => println!("Could not find {}", process_name),
    }
}

pub fn start_hijacked_process(proc_path: &str, module_path: &str) {
    unsafe {
        let mut lp_startup_info: MaybeUninit<STARTUPINFOW> = MaybeUninit::zeroed();
        lp_startup_info.assume_init().cb = size_of::<STARTUPINFOW>() as _;
        let mut lp_process_information: MaybeUninit<PROCESS_INFORMATION> = MaybeUninit::zeroed();

        let path = fs::canonicalize(Path::new(proc_path)).unwrap();
        let mut proc_path_buf = path.as_os_str().encode_wide().collect::<Vec<u16>>();
        // Add null terminator
        proc_path_buf.push(0);

        enable_debug_priv();
        CreateProcessW(
            PWSTR(proc_path_buf.as_mut_ptr()),
            None,
            None,
            None,
            false,
            CREATE_NEW_CONSOLE,
            None,
            None,
            lp_startup_info.as_mut_ptr(),
            lp_process_information.as_mut_ptr(),
        ).unwrap();
        
        let h_process = lp_process_information.assume_init().hProcess;
        inject_payload(h_process, module_path);
    };
}