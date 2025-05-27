{
  description = "CyTrans -- a command line- and web-based transcoder for CyTube";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url="github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows="nixpkgs";
  };

  outputs = { self, nixpkgs, fenix, flake-utils }: 

  flake-utils.lib.eachDefaultSystem (system:
  let
    pkgs = nixpkgs.legacyPackages.${system};
    fenixPkgs = fenix.packages.${system};
    fenixToolchain = fenixPkgs.fromToolchainFile {
      file=./rust-toolchain.toml;
      sha256="sha256-pw28Lw1M3clAtMjkE/wry0WopX0qvzxeKaPUFoupC00=";
    };
  in {
    devShells.default = pkgs.mkShell {
      buildInputs = [fenixToolchain pkgs.wasm-pack];
    };

    packages = {
      cytrans-web-client = pkgs.runCommand "cytrans-web-client" {
        buildInputs = [fenixToolchain pkgs.wasm-pack];
      } ''
        cd ${./cytrans-web/client}
	wasm-pack build --target web --out-dir $out
      '';
    };

    packages.default = pkgs.hello;

  });
}
