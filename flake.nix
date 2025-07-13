{
  inputs = {
    # cargo2nix.url = "path:../../";
    # Use a github flake URL for real packages
    cargo2nix.url = "github:cargo2nix/cargo2nix/42d9bc527ff2bc90cc52024f66bb48d9b2a9e9dc";
    flake-utils.follows = "cargo2nix/flake-utils";
    nixpkgs.follows = "cargo2nix/nixpkgs";
    # crane = {
    #   url = "github:ipetkov/crane";
    #   inputs = {
    #     nixpkgs.follows = "nixpkgs";
    #     rust-overlay.follows = "rust-overlay";
    #     flake-utils.follows = "flake-utils";
    #   };
    # };

  };

  outputs = inputs: with inputs; # pass through all inputs and bring them into scope

    # Build the output set for each default system and map system sets into
    # attributes, resulting in paths such as:
    # nix build .#packages.x86_64-linux.<name>
    flake-utils.lib.eachDefaultSystem (system:

      # let-in expressions, very similar to Rust's let bindings.  These names
      # are used to express the output but not themselves paths in the output.
      let

        # create nixpkgs that contains rustBuilder from cargo2nix overlay
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
        };

        # create the workspace & dependencies package set
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustVersion = "1.85.0";
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [

        # parentheses disambiguate each makeOverride call as a single list element
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "alloc-no-stdlib";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.clang
                pkgs.libclang
                pkgs.glibc
                pkgs.lld
                ];
            };
        })
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "yaml-rust2";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.clang
                pkgs.libclang
                pkgs.glibc
                pkgs.lld
                ];
            };
        })
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "clap";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.clang
                pkgs.libclang
                pkgs.glibc
                pkgs.lld
                ];
            };
        })
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "zopfli";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.clang
                pkgs.libclang
                pkgs.glibc
                pkgs.lld
                ];
            };
        })
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "utoipa-swagger-ui";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.curl
                ];
            };
        })
        (pkgs.rustBuilder.rustLib.makeOverride {
            name = "poros";
            overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                pkgs.flatbuffers
                ];
            };
        })
        ];
        };

      in rec {
        # this is the output (recursive) set (expressed for each system)

        # the packages in `nix build .#packages.<system>.<name>`
        packages = {
          # nix build .#poros
          # nix build .#packages.x86_64-linux.poros
          poros = (rustPkgs.workspace.poros {});
          # nix build
          default = packages.poros; # rec
        };
      }
    );
}

