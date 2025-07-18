{pkgs ? import <nixos-unstable> {}}:
pkgs.mkShell {
  shellHook = ''
  '';

  nativeBuildInputs = with pkgs.buildPackages; [
    rustc
    cargo
    rust-analyzer

    # glibc.static
    pkg-config
    openssl

    # needed for rns
    protobuf
  ];
}
