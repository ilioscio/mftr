# The MFTR dedicated server and tools (`mftr-server`, `mftr-tools`), built from this checkout.
# The Godot extension and client aren't part of it.
{ lib, rustPlatform }:

let
  workspace = (lib.importTOML ../Cargo.toml).workspace.package;
in
rustPlatform.buildRustPackage {
  pname = "mftr-server";
  inherit (workspace) version;

  # Only what cargo needs, so editing docs or the Godot client doesn't rebuild the server.
  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
    ];
  };
  # Every dependency is pinned with its checksum in Cargo.lock: no vendor hash to update.
  cargoLock.lockFile = ../Cargo.lock;

  # The profile we ship (stripped, thin LTO); see Cargo.toml.
  buildType = "dist";
  cargoBuildFlags = [ "-p" "mftr-server" "-p" "mftr-tools" ];
  # The transport and wire-format tests are quick; the full suite runs in CI.
  cargoTestFlags = [ "-p" "mftr-net" ];

  meta = {
    description = "MFTR authoritative match server and tools";
    homepage = "https://github.com/ilioscio/mftr";
    license = lib.licenses.agpl3Plus;
    mainProgram = "mftr-server";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
