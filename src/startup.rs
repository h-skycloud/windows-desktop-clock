use std::{env, io, os::windows::ffi::OsStrExt, ptr::null_mut};
use windows_sys::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::*,
    },
    w,
};

const RUN: *const u16 = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const NAME: *const u16 = w!("MasaustuSaati");

pub fn enabled() -> io::Result<bool> {
    let mut length = 0;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            NAME,
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut length,
        )
    };
    match status {
        ERROR_SUCCESS => Ok(true),
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(false),
        _ => Err(io::Error::from_raw_os_error(status as i32)),
    }
}

pub fn set(enabled: bool) -> io::Result<()> {
    let status = if enabled {
        let exe = env::current_exe()?;
        let command: Vec<u16> = [u16::from(b'"')]
            .into_iter()
            .chain(exe.as_os_str().encode_wide())
            .chain([u16::from(b'"'), 0])
            .collect();
        if command.len() > 261 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The executable path is too long.",
            ));
        }
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN,
                NAME,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, NAME) } {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ERROR_SUCCESS,
            status => status,
        }
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}
