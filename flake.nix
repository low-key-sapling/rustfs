# Nix flake for building ZfFS
#
# Prerequisites:
#   Install Nix: https://nixos.org/download/
#   Enable flakes: https://nixos.wiki/wiki/Flakes#Enable_flakes
#
# Usage:
#   nix build          # Build zffs binary
#   nix run            # Build and run zffs
#   ./result/bin/zffs --help
{
  description = "ZfFS - S3-compatible object storage based on RustFS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      productVersion = builtins.replaceStrings [ "\n" "\r" ] [ "" "" ] (builtins.readFile ./ZFFS_VERSION);
    in
    {
      packages = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };

          # Use the latest stable rust toolchain
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
              "rustfmt"
            ];
          };

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "zffs";
            version = productVersion;

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              protobuf
            ];

            buildInputs = with pkgs; [
              openssl
            ];

            cargoBuildFlags = [
              "--package"
              "rustfs"
            ];

            # Set environment variables for build
            PROTOC = "${pkgs.protobuf}/bin/protoc";
            ZFFS_VERSION = productVersion;

            doCheck = false;

            meta = {
              description = "High-performance S3-compatible object storage";
              homepage = "https://rustfs.com";
              license = pkgs.lib.licenses.asl20;
              mainProgram = "zffs";
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
              "rustfmt"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.pkg-config
              pkgs.protobuf
              pkgs.openssl
            ];

            PROTOC = "${pkgs.protobuf}/bin/protoc";
          };
        }
      );
    };
}
