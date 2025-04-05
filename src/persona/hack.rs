use sysinfo::System;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, MODULEENTRY32, Module32First, TH32CS_SNAPMODULE,
};

pub fn find_process_id(name: &str) -> Option<u32> {
    let mut sys = System::new_all();

    sys.refresh_all();

    for (pid, process) in sys.processes() {
        if process.name() == name {
            return Some(pid.as_u32());
        }
    }

    None
}

pub fn get_base_address(pid: u32) -> Option<usize> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid).ok()?;
        let mut module_entry = MODULEENTRY32::default();
        module_entry.dwSize = std::mem::size_of::<MODULEENTRY32>() as u32;

        if Module32First(snapshot, &mut module_entry).is_ok() {
            Some(module_entry.modBaseAddr as usize)
        } else {
            None
        }
    }
}
