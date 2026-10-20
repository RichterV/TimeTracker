/// Segundos desde a última entrada de teclado ou mouse.
#[cfg(windows)]
pub fn segundos_ocioso() -> i64 {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if GetLastInputInfo(&mut info) == 0 {
            return 0;
        }
        (GetTickCount().wrapping_sub(info.dwTime) / 1000) as i64
    }
}

#[cfg(not(windows))]
pub fn segundos_ocioso() -> i64 {
    0
}
