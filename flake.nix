{
  description = "CyTrans -- a TUI- and web-based transcoder for CyTube";

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
    cytrans-web-cargo-nix = (import ./cytrans-web/Cargo.nix {inherit pkgs;});
    cytrans-web-server = cytrans-web-cargo-nix.workspaceMembers.server.build;
      #.internal.buildRustCrateWithFeatures {packageId="anyhow";};
    cytrans-web-client = (import ./cytrans-web/Cargo.nix {pkgs = pkgs.pkgsCross.wasm32-unknown-none;}).workspaceMembers.client.build.lib;

    # wasm-bindgen-cli must be *exactly* the same version as the wasm-bindgen version used by our crate,
    # so to be resilient in the face of us updating the version of wasm-bindgen we use, we pin it to the same version.
    # we will have to update two hashes every time but I call that a fair trade.
    wasm-bindgen-cli = pkgs.buildWasmBindgenCli rec {
      src = pkgs.fetchCrate {
        pname = "wasm-bindgen-cli";
	version = cytrans-web-cargo-nix.internal.crates.wasm-bindgen.version;
	hash = "sha256-3RJzK7mkYFrs7C/WkhW9Rr4LdP5ofb2FdYGz1P7Uxog=";
      };
      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit src;
	inherit (src) pname version;
	hash = "sha256-qsO12332HSjWCVKtf1cUePWWb9IdYUmT+8OPj/XP2WE=";
      };
    };
  in {
    devShells.default = pkgs.mkShell {
      buildInputs = [fenixToolchain wasm-bindgen-cli];
    };

    packages = {
      inherit cytrans-web-server cytrans-web-client;
      cytrans-web-www = pkgs.runCommand "cytrans-web-www-root" {nativeBuildInputs = [wasm-bindgen-cli pkgs.binaryen];} ''
      shopt -s extglob
      echo Optimizing wasm...
      wasm-opt ${cytrans-web-client}/lib/client*.wasm -o client.wasm
      mkdir $out
      ln -s ${./cytrans-web/www}/!(client*) $out/
      echo Generating JS bindings...
      wasm-bindgen client.wasm --target web --no-typescript --out-dir $out/client
      '';
    };

  });
}
