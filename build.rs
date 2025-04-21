use std::io;

fn main() -> io::Result<()> {
    let mut binding = winres::WindowsResource::new();
    let win_res = binding
        .set_icon("src/assets/veritas.ico");

    #[cfg(not(debug_assertions))]
    win_res
        .set_manifest(r#"
        <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
        <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
            <security>
                <requestedPrivileges>
                    <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
                </requestedPrivileges>
            </security>
        </trustInfo>
        </assembly>
        "#);

    win_res.compile()
}