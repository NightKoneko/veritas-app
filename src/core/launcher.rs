use std::{
    fs, mem::{self}, os::windows::ffi::OsStrExt, path::Path
};

use windows::{core::{s, PWSTR}, Win32::{Foundation::{CloseHandle, HANDLE, LUID}, Security::LUID_AND_ATTRIBUTES, System::{Diagnostics::Debug::WriteProcessMemory, LibraryLoader::{GetModuleHandleW, GetProcAddress}, Memory::{self, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE}, Threading::{CreateRemoteThread, ResumeThread, WaitForSingleObject, CREATE_NEW_CONSOLE, CREATE_SUSPENDED, INFINITE, PROCESS_INFORMATION, STARTUPINFOW}}}};
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
        let mut h_token = HANDLE::default();
        let mut luid = LUID::default();
        let tkp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        match OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut h_token,
        ) {
            Ok(_) => {
                LookupPrivilegeValueW(
                    None,
                    SE_DEBUG_NAME,
                    &mut luid
                ).unwrap();

                AdjustTokenPrivileges(
                    h_token,
                    false,
                    Some(&tkp),
                    mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                    None,
                    None,
                ).unwrap();
        
                CloseHandle(h_token).unwrap();        
            },
            Err(e) => println!("{}", e)
        }
    }
}

fn find_process(process_name: &str) -> Option<HANDLE> {
    unsafe {
        let process_name = process_name.to_lowercase();
        let mut entry = PROCESSENTRY32W {
            dwSize: mem::size_of::<PROCESSENTRY32W>() as _,
            ..Default::default()
        };

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();

        if Process32FirstW(snapshot, &mut entry as _).is_ok() {
            loop {
                let proc_name = String::from_utf16_lossy(entry.szExeFile.as_slice());
                // Makes it case insensitive
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
        let path = std::path::absolute(module_path).unwrap();
        let module_path_buf = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>();
        // # of bytes
        let len = module_path_buf.len() * size_of::<u16>();

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
        let proc_path = Path::new(proc_path);
        let dir = proc_path.parent().unwrap_or(Path::new("."));
        
        let mut proc_path_buf = proc_path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>();

        let mut dir_buf = dir
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>();

        let mut startup_info = STARTUPINFOW {
            cb: mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut process_info = PROCESS_INFORMATION::default();

        let result = CreateProcessW(
            PWSTR(proc_path_buf.as_mut_ptr()),
            None,
            None,
            None,
            false,
            CREATE_SUSPENDED, 
            None,
            PWSTR(dir_buf.as_mut_ptr()),
            &mut startup_info,
            &mut process_info,
        );

        match result {
            Ok(_) => {
                println!("Process created, waiting before injection...");
                std::thread::sleep(std::time::Duration::from_millis(1000));

                let process = Threading::OpenProcess(
                    PROCESS_ALL_ACCESS,
                    false,
                    process_info.dwProcessId
                ).unwrap();

                let path_str = std::path::absolute(module_path).unwrap();
                let dll_path = path_str.to_str().unwrap();
                let path_len = dll_path.len() + 1;

                let allocation = Memory::VirtualAllocEx(
                    process,
                    None,
                    path_len,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE
                );

                println!("Allocated memory for DLL path at {:?}", allocation);

                WriteProcessMemory(
                    process,
                    allocation,
                    dll_path.as_ptr() as _,
                    path_len,
                    None
                ).unwrap();

                let kernel32 = GetModuleHandleW(w!("kernel32")).unwrap();
                let load_library = GetProcAddress(kernel32, s!("LoadLibraryA")).unwrap();
                
                println!("LoadLibraryA found at {:?}", load_library);

                let thread = CreateRemoteThread(
                    process,
                    None,
                    0,
                    Some(std::mem::transmute(load_library)),
                    Some(allocation),
                    0,
                    None
                ).unwrap();

                WaitForSingleObject(thread, INFINITE);
                CloseHandle(thread);

                println!("DLL injected, resuming process...");
                ResumeThread(process_info.hThread);

                CloseHandle(process);
                CloseHandle(process_info.hThread);
                CloseHandle(process_info.hProcess);
                
                println!("Game launch complete");
            },
            Err(e) => println!("Failed to create process: {}", e),
        }
    }
}