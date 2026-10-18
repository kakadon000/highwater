{
  lib,
  rustPlatform,
  pkg-config,
  cmake,
  git,
  clang,
  llvmPackages,
  patchelf,
  fontconfig,
  libxkbcommon,
  wayland,
  libGL,
  xorg,
  sqlite,
  dbus,
  hicolor-icon-theme,
}:
let
  manifest = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  # Loaded with dlopen by the client, so no linker sees them: winit's Wayland, X11 and
  # keyboard libraries, and the GL loader Slint's OpenGL renderer draws through.
  runtimeLibraries = [
    libxkbcommon
    wayland
    libGL
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
  ];
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "highwater";
  version = manifest.workspace.package.version;
  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [
    "--workspace"
    "--bins"
  ];
  doCheck = false;
  dontCargoInstall = true;

  installPhase = ''
    runHook preInstall
    runHook postInstall
  '';

  nativeBuildInputs = [
    pkg-config
    cmake
    git
    clang
    llvmPackages.libclang
    patchelf
  ];
  buildInputs = [
    fontconfig
    sqlite
    dbus
    hicolor-icon-theme
  ];
  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

  postInstall = ''
    install -Dm755 target/*/release/highwater -t "$out/bin"
    install -Dm755 target/*/release/highwaterd -t "$out/bin"
    install -Dm755 target/*/release/highwaterctl -t "$out/bin"
    install -Dm644 data/applications/io.github.kakadon000.Highwater.desktop -t "$out/share/applications"
    install -Dm644 data/metainfo/io.github.kakadon000.Highwater.metainfo.xml -t "$out/share/metainfo"
    install -d "$out/share/icons"
    cp -r data/icons/hicolor "$out/share/icons/"
    install -Dm644 data/dbus-1/services/io.github.kakadon000.Highwater.Daemon.service -t "$out/share/dbus-1/services"
    install -Dm644 data/highwaterd.service -t "$out/lib/systemd/user"
    substituteInPlace "$out/share/dbus-1/services/io.github.kakadon000.Highwater.Daemon.service" \
      --replace-fail /usr/bin/highwaterd "$out/bin/highwaterd"
    substituteInPlace "$out/lib/systemd/user/highwaterd.service" \
      --replace-fail /usr/bin/highwaterd "$out/bin/highwaterd"
  '';

  postFixup = ''
    patchelf --add-rpath "${lib.makeLibraryPath runtimeLibraries}" "$out/bin/highwater"
  '';

  meta = {
    description = "Track AI provider quota limits";
    homepage = "https://github.com/kakadon000/highwater";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "highwater";
  };
})
