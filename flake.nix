{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ self, nixpkgs, ... }:
    let
      # forAllSystems follows this guide:
      # https://github.com/Misterio77/nix-starter-configs/issues/64#issuecomment-1941420712
      pkgsFor =
        system:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.fenix.overlays.default
            ];
          };
          rustToolchain = pkgs.fenix.fromToolchainFile {
            dir = ./.;
            sha256 = "sha256-zm3dyIY2T414ZRR3EhLOvptzG6gta4WZUcawzMUWtqI=";
          };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        in
        {
          inherit
            system
            pkgs
            rustToolchain
            rustPlatform
            ;
        };

      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      common = {
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (pkgsFor system));
    in
    {
      devShells = forAllSystems (
        { pkgs, rustToolchain, ... }:
        {
          default =
            with pkgs;
            mkShell {
              packages = [
                nix
                rustToolchain
                git
                mdbook
              ];
            };
        }
      );
      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt-tree);
      packages = forAllSystems (
        { pkgs, rustPlatform, ... }:
        {
          default = pkgs.rustPlatform.buildRustPackage (
            common
            // {
              pname = "mdbook-xref";
            }
          );

          mdbook-xref = rustPlatform.buildRustPackage (
            common
            // {
              pname = "mdbook-xref";
              version = "0.2.0";
              cargoBuildFlags = [
                "--bin"
                "mdbook-xref"
              ];
            }
          );

          mdbook-figure = rustPlatform.buildRustPackage (
            common
            // {
              pname = "mdbook-figure";
              version = "0.2.0";
              cargoBuildFlags = [
                "--bin"
                "mdbook-figure"
              ];
            }
          );

          mdbook-abbr2 = rustPlatform.buildRustPackage (
            common
            // {
              pname = "mdbook-abbr2";
              version = "0.1.0";
              cargoBuildFlags = [
                "--bin"
                "mdbook-abbr2"
              ];
            }
          );
        }
      );
    };
}
