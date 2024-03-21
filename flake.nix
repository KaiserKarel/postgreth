{
  description = "Postgreth is Ethereum on Postgres";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix.url = "github:numtide/treefmt-nix";

  };
  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
      ];
      flake = {
        lib = {
          buildPostgrethExtension = pkgs: postgresql: (pkgs.buildPgrxExtension {
            inherit postgresql;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            name = "postgreth";
            doCheck = false;
          });
        };
      };
      systems =
        [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      perSystem = { self', system, lib, config, pkgs, ... }: {
        packages = {
          postgreth_14 = self.lib.buildPostgrethExtension pkgs pkgs.postgresql_14;
          postgreth_15 = self.lib.buildPostgrethExtension pkgs pkgs.postgresql_15;
          postgreth_16 = self.lib.buildPostgrethExtension pkgs pkgs.postgresql_16;
        };
        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };
      };
    };
}
