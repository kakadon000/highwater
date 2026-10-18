{
  description = "Highwater desktop quota tracker";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      packageFor =
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.callPackage ./nix/package.nix { };
    in
    {
      packages = forAllSystems (
        system:
        let
          package = packageFor system;
        in
        {
          default = package;
          highwater = package;
        }
      );

      apps = forAllSystems (
        system:
        let
          package = packageFor system;
        in
        {
          default = {
            type = "app";
            program = "${package}/bin/highwater";
          };
          highwater = {
            type = "app";
            program = "${package}/bin/highwater";
          };
          highwaterd = {
            type = "app";
            program = "${package}/bin/highwaterd";
          };
        }
      );

      nixosModules.default = import ./nix/module.nix { inherit self; };

      checks = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          evaluated = nixpkgs.lib.nixosSystem {
            inherit system;
            modules = [
              self.nixosModules.default
              { services.highwater.enable = true; }
            ];
          };
          service = evaluated.config.systemd.user.services.highwaterd;
        in
        assert service.wantedBy == [ ];
        assert service.serviceConfig.Type == "dbus";
        assert service.serviceConfig.BusName == "io.github.kakadon000.Highwater.Daemon";
        assert
          service.serviceConfig.ExecStart == "${evaluated.config.services.highwater.package}/bin/highwaterd";
        {
          nixos-module = pkgs.runCommandNoCC "highwater-nixos-module-evaluation" { } "touch $out";
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
              pkgs.pkg-config
              pkgs.cmake
              pkgs.clang
              pkgs.llvmPackages.libclang
              pkgs.fontconfig
              pkgs.sqlite
              pkgs.dbus
              pkgs.desktop-file-utils
              pkgs.appstream
              pkgs.shellcheck
            ];
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            # What the client opens with dlopen at startup; see nix/package.nix.
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              pkgs.libxkbcommon
              pkgs.wayland
              pkgs.libGL
              pkgs.xorg.libX11
              pkgs.xorg.libXcursor
              pkgs.xorg.libXi
              pkgs.xorg.libXrandr
            ];
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt-rfc-style
      );
    };
}
