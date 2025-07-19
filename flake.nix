{
  description = "Dev shell for rns-dns";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05"; # or stable if you prefer

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
          };
        }
      );
    in
    {
      devShells = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustc
              cargo
              rust-analyzer

              # glibc.static
              pkg-config
              openssl

              # needed for rns
              protobuf
              rns           
            ];

            # nativeBuildInputs = with pkgs;
              # lib.optionals stdenv.isLinux [ xorg.libX11.dev ]
              # ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Cocoa ];
          };
        });
    };
}
