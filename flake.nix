{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      # self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          # crossSystem = {
          #   config = "x86_64-unknown-linux-gnu";
          #   rustc.config = "x86_64-unknown-linux-gnu";
          # };
        };
        rustVersion = "1.83.0";
        myRust = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [
            "rust-src" # for rust-analyzer
            "rust-analyzer"
          ];
          targets = [
            "x86_64-unknown-linux-gnu"
            "aarch64-apple-darwin"
          ];
        };

        # rustPlatform = pkgs.makeRustPlatform {
        #   cargo = rustToolchain;
        #   rustc = rustToolchain;
        # };

      in
      with pkgs;
      {
        devShells.default = mkShell {
          packages =
            [
              myRust
              # rustPlatform.rust
              cmake
              pkg-config
              protobuf
              libiconv
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
              (with pkgs.darwin.apple_sdk.frameworks; [
                Security
                SystemConfiguration
                libiconv
              ])
            ];
        };

        formatter = nixpkgs-fmt;
      }
    );
}
