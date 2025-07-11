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
      sha256="sha256-eFuFA5spScrde7b7lSV5QAND1m0+Ds6gbVODfDE3scg=";
    };
    buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
      cargo=fenixToolchain;
      rustc=fenixToolchain;
    };
    cytrans-web-cargo-nix = (import ./cytrans-web/Cargo.nix {inherit pkgs buildRustCrateForPkgs;});
    #cytrans-web-server = cytrans-web-cargo-nix.workspaceMembers.server.build;
    cytrans-web-server = cytrans-web-cargo-nix.workspaceMembers.server-ng.build;
      #.internal.buildRustCrateWithFeatures {packageId="anyhow";};
    cytrans-web-client-wasm = (import ./cytrans-web/Cargo.nix {pkgs = pkgs.pkgsCross.wasm32-unknown-none; inherit buildRustCrateForPkgs;}).workspaceMembers.client.build.lib;
    cytrans-web-client = doWasmBindgen cytrans-web-client-wasm "client";

    cytrans-tui-cargo-nix = (import ./cytrans-cli/Cargo.nix {inherit buildRustCrateForPkgs; pkgs=pkgs.pkgsCross.musl64.pkgsStatic;});

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

    doWasmBindgen = drv: outName: pkgs.runCommand "${outName}-bindgen" {buildInputs=[wasm-bindgen-cli pkgs.binaryen];} ''
      wasm-opt ${drv}/lib/*.wasm -o ${outName}.wasm # *.wasm should match exactly one file
      mkdir $out
      wasm-bindgen ${outName}.wasm --target web --no-typescript --out-dir $out/${outName}
    '';

    brotlifyScriptInternal = pkgs.writeShellScript "brotlify_script" ''
      OUT_DIR="$1"
      IN_PATH="$2"
      REL_PATH=$(awk -F // "{print \$2}" <<< $IN_PATH)
      OUT_PATH="$OUT_DIR/$REL_PATH.br"
      mkdir -p $(dirname "$OUT_PATH")
      ${pkgs.brotli}/bin/brotli -9 "$IN_PATH" -o "$OUT_PATH"
    '';

    cytrans-web-www-compressed = pkgs.runCommand "cytrans-www-compressed" {} ''
      shopt -s extglob
      find ${./cytrans-web/www}//!(client*) ${cytrans-web-client}// -type f -print0 | xargs -0 -n 1 -P $NIX_BUILD_CORES ${brotlifyScriptInternal} $out
    '';
  in {
    inherit fenixToolchain;
    devShells.default = pkgs.mkShell {
      buildInputs = [fenixToolchain wasm-bindgen-cli pkgs.crate2nix];
    };

    packages = {
      inherit cytrans-web-server cytrans-web-client;
      /*
      cytrans-web-www = pkgs.runCommand "cytrans-web-www-root" {nativeBuildInputs = [wasm-bindgen-cli pkgs.binaryen];} ''
      shopt -s extglob
      echo Optimizing wasm...
      wasm-opt ${cytrans-web-client}/lib/client*.wasm -o client.wasm
      mkdir $out
      ln -s ${./cytrans-web/www}/!(client*) $out/
      echo Generating JS bindings...
      wasm-bindgen client.wasm --target web --no-typescript --out-dir $out/client
      '';
      */
      inherit cytrans-web-www-compressed;
      cytrans-tui = cytrans-tui-cargo-nix.rootCrate.build;
    };

  });
}
