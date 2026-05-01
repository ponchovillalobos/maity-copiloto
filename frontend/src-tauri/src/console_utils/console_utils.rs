#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn AllocConsole() -> i32;
    #[allow(dead_code)]
    fn FreeConsole() -> i32;
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
    fn ShowWindow(hwnd: *mut std::ffi::c_void, n_cmd_show: i32) -> i32;
}

#[cfg(target_os = "windows")]
extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn SetStdHandle(nStdHandle: u32, hHandle: *mut std::ffi::c_void) -> i32;
}

#[cfg(target_os = "windows")]
const SW_HIDE: i32 = 0;
#[cfg(target_os = "windows")]
const SW_SHOW: i32 = 5;

/// Redirect stdout/stderr to the newly allocated console.
/// Updates the process standard handles so future I/O operations target the console.
#[cfg(target_os = "windows")]
unsafe fn redirect_std_to_console() {
    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const OPEN_EXISTING: u32 = 3;
    const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1isize as *mut std::ffi::c_void;
    const STD_OUTPUT_HANDLE: u32 = (-11i32) as u32;
    const STD_ERROR_HANDLE: u32 = (-12i32) as u32;

    let conout: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
    let handle = CreateFileW(
        conout.as_ptr(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        ptr::null(),
        OPEN_EXISTING,
        0,
        ptr::null(),
    );

    if handle != INVALID_HANDLE_VALUE && handle != ptr::null_mut() {
        SetStdHandle(STD_OUTPUT_HANDLE, handle);
        SetStdHandle(STD_ERROR_HANDLE, handle);
    }
}

#[tauri::command]
pub fn show_console() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window == ptr::null_mut() {
            // If no console exists, allocate one
            if AllocConsole() == 0 {
                return Err("Failed to allocate console".to_string());
            }
            // Redirect stdout/stderr to the new console via Win32 API
            // Note: For println! to work, AllocConsole should ideally be called
            // before any I/O initialization (see main.rs auto-allocation).
            redirect_std_to_console();
        } else {
            // Show existing console window
            ShowWindow(console_window, SW_SHOW);
        }
        Ok("Console shown".to_string())
    }
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, we'll open Terminal.app with our app's logs
        // First, get the app name from the bundle
        match Command::new("osascript")
            .arg("-e")
            .arg(r#"
                tell application "Terminal"
                    activate
                    do script "log stream --process maity-copiloto --level info --style compact"
                end tell
            "#)
            .spawn()
        {
            Ok(_) => Ok("Console opened in Terminal".to_string()),
            Err(e) => Err(format!("Failed to open console: {}", e)),
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok("Console control is only available on Windows and macOS".to_string())
    }
}

#[tauri::command]
pub fn hide_console() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window != ptr::null_mut() {
            ShowWindow(console_window, SW_HIDE);
            Ok("Console hidden".to_string())
        } else {
            Err("No console window found".to_string())
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, we'll close the Terminal window that's showing our logs
        match Command::new("osascript")
            .arg("-e")
            .arg(r#"
                tell application "Terminal"
                    set windowList to windows
                    repeat with aWindow in windowList
                        if contents of selected tab of aWindow contains "log stream --process maity-copiloto" then
                            close aWindow
                        end if
                    end repeat
                end tell
            "#)
            .spawn()
        {
            Ok(_) => Ok("Console closed".to_string()),
            Err(e) => Err(format!("Failed to close console: {}", e)),
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok("Console control is only available on Windows and macOS".to_string())
    }
}

#[tauri::command]
pub fn toggle_console() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window == ptr::null_mut() {
            show_console()
        } else {
            // Check if window is visible (this is a simplified approach)
            // In a real implementation, you might want to use GetWindowLong to check visibility
            hide_console()
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, check if Terminal is running with our log stream
        let check_result = Command::new("osascript")
            .arg("-e")
            .arg(r#"
                tell application "Terminal"
                    set windowList to windows
                    repeat with aWindow in windowList
                        if contents of selected tab of aWindow contains "log stream --process maity-copiloto" then
                            return "found"
                        end if
                    end repeat
                    return "not found"
                end tell
            "#)
            .output();
            
        match check_result {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.trim() == "found" {
                    hide_console()
                } else {
                    show_console()
                }
            }
            Err(_) => show_console()
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok("Console control is only available on Windows and macOS".to_string())
    }
}