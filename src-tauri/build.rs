fn main() {
    // The app always starts as administrator (the `requireAdministrator`
    // manifest): Windows shows the UAC prompt when opening it and, if the user
    // rejects it, the app does not start. This way `yt-dlp -U` (launched at 50%
    // of the splash) can overwrite the binary in the install path without
    // extra elevation.
    //
    // Note: keep the Common Controls v6 dependency — without it the native rfd
    // dialogs (select_folder, select_media_file...) break.
    let mut windows = tauri_build::WindowsAttributes::new();
    windows = windows.app_manifest(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#,
    );
    let attrs = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attrs).expect("failed to run build script");
}
