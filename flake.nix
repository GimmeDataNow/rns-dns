{
  description = "Dev shell for rns-dns";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05"; # or stable if you prefer
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
          config = {
            allowUnfree = true;
            permittedInsecurePackages = [
              "python3.12-ecdsa-0.19.1"
            ];
          };
        }
      );
    in
    {
      devShells = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              (rust-bin.nightly.latest.default.override {
                extensions = [ "rust-src" ];
              })
              rust-analyzer
              # glibc.static
              pkg-config
              openssl
              # needed for rns
              protobuf
              rns           
            ];
            shellHook = ''
              # 🔄 Replace './tmux.conf' with the path to your configuration file
              # source ~/.bashrc
              # exec ./tmux.sh
            '';
            # nativeBuildInputs = with pkgs;
              # lib.optionals stdenv.isLinux [ xorg.libX11.dev ]
              # ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Cocoa ];
          };
        });
    };
}
