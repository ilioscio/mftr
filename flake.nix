{
  description = "MFTR (Moba For The Rest of us): development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        }));
    in
    {
      devShells = forAllSystems (pkgs:
        let
          # Same toolchain as everyone else: rust-toolchain.toml at the repo root.
          rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          # The extension targets the Godot 4.5 API (forward-compatible), so any 4.5+ works.
          # Prefer the newest versioned attribute nixpkgs has.
          godot =
            if pkgs ? godot_4_7 then pkgs.godot_4_7
            else if pkgs ? godot_4_6 then pkgs.godot_4_6
            else if pkgs ? godot_4_5 then pkgs.godot_4_5
            else pkgs.godot;

          # Libraries Godot (and anything else we dlopen) needs at runtime on NixOS.
          runtimeLibs = with pkgs; [
            vulkan-loader
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXext
            xorg.libXi
            xorg.libXinerama
            xorg.libXrandr
            alsa-lib
            libpulseaudio
            udev
            fontconfig
            dbus
          ];
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              godot
              pkgs.pkg-config
              pkgs.gh
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
            shellHook = ''
              export MFTR_GODOT="$(command -v godot || command -v godot4)"
              echo "MFTR dev shell: $(rustc --version)"
              echo "  Godot: $("$MFTR_GODOT" --version 2>/dev/null || echo 'not found')  ($MFTR_GODOT)"
            '';
          };
        });
    };
}
