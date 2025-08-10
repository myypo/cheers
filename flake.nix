{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    pre-commit-hooks.url = "github:myypo/git-hooks.nix";
    pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs:
    let
      forEachSupportedSystem =
        let
          supportedSystems = [
            "x86_64-linux"
          ];
        in
        (
          f:
          inputs.nixpkgs.lib.genAttrs supportedSystems (
            system:
            f {
              pkgs =
                let
                  overlays = [ inputs.fenix.overlays.default ];
                in
                import inputs.nixpkgs { inherit overlays system; };

              pre-commit-hooks = inputs.pre-commit-hooks.lib.${system}.run;
              pre-commit-check = inputs.self.checks.${system}.pre-commit-check;
            }
          )
        );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs, pre-commit-check, ... }:
        {
          default = pkgs.mkShell {
            env = {
              LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [ pkgs.openssl ]}";
            };

            packages = with pkgs; [
              openssl
              pkg-config

              cargo-machete

              rust-analyzer-nightly
              (
                with fenix;
                with complete;
                combine [
                  cargo
                  clippy
                  rust-src
                  rustc
                  rustfmt
                ]
              )
            ];

            inherit (pre-commit-check) shellHook;
            buildInputs = pre-commit-check.enabledPackages;
          };
        }
      );

      checks = forEachSupportedSystem (
        { pkgs, pre-commit-hooks, ... }:
        let
          testFlagsMatrix = [
            ""
            "-F skip_dev_check"
            "--release"
            "--release -F skip_dev_check"
          ];

          testHooks = builtins.listToAttrs (
            builtins.map (flags: {
              name = "tests-(${flags})";
              value = {
                enable = true;
                name = "Unit and integration tests (${flags})";
                entry = "cargo test --workspace ${flags}";
                pass_filenames = false;
              };
            }) testFlagsMatrix
          );
        in
        {
          pre-commit-check =
            let
              default_stages = [
                "pre-push"
                "manual"
              ];
            in
            pre-commit-hooks {
              src = ./.;
              inherit default_stages;
              hooks = testHooks // {
                nixfmt-rfc-style.enable = true;
                typos = {
                  enable = true;
                  stages = default_stages ++ [ "commit-msg" ];
                };
                taplo.enable = true;
                actionlint.enable = true;

                clippy = {
                  enable = true;
                  settings.allFeatures = true;
                  settings.denyWarnings = true;
                };
                rustfmt.enable = true;

              };
              settings = {
                rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
                  lockFile = ./Cargo.lock;
                };
              };
            };
        }
      );
    };
}
