{
  description = "mdbook-backlinks";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
      in
      {
        packages.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
        };
      }
    );
}
