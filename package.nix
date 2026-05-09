{
  openssl,
  pkg-config,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "karatui";
  version = "0.1.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ];
}
