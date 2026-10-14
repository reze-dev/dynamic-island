{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "isled";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ../core;
    fileset = lib.fileset.unions [
      ../core/Cargo.toml
      ../core/Cargo.lock
      ../core/isled
      ../core/isle-core
    ];
  };

  cargoLock.lockFile = ../core/Cargo.lock;

  meta = {
    description = "Session daemon for the Dynamic Island Shell";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
    mainProgram = "isled";
  };
}
