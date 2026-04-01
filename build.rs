// Build script for WinSH
// Handles build configuration

fn main() {
    // Note: WinuxCmd FFI now uses dynamic loading via libloading
    // No static linking configuration needed

    #[cfg(debug_assertions)]
    {
        // Check for development mode warnings
        // Note: DLL name changed from winuxcmd.dll to winuxcore.dll
        let dll_path = std::path::Path::new("utils/winuxcmd/winuxcore.dll");
        if !dll_path.exists() {
            println!(
                "cargo:warning=winuxcore.dll not found at {} for development",
                dll_path.display()
            );
        }
    }
}
