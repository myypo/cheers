{
  inputs = {
    # Needed due to vendoring datastar
    self.submodules = true;

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
              self = inputs.self;
              pkgs =
                let
                  overlays = [ inputs.fenix.overlays.default ];
                in
                import inputs.nixpkgs { inherit overlays system; };
              pre-commit-hooks = inputs.pre-commit-hooks.lib.${system}.run;
              pre-commit-check = inputs.self.checks.${system}.pre-commit-check;
              rust-toolchain = inputs.fenix;
            }
          )
        );
    in
    {
      devShells = forEachSupportedSystem (
        {
          self,
          pkgs,
          rust-toolchain,
          pre-commit-check,
          ...
        }:
        {
          default = pkgs.mkShell {
            env = {
              LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [ pkgs.openssl ]}";
            };

            packages = with pkgs; [
              openssl
              pkg-config

              act

              cargo-deny
              cargo-machete

              rust-analyzer-nightly

              inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.default
            ];

            inherit (pre-commit-check) shellHook;
            buildInputs = pre-commit-check.enabledPackages;
          };
        }
      );

      checks = forEachSupportedSystem (
        { pkgs, pre-commit-hooks, ... }:
        let
          testHooks = builtins.listToAttrs (
            builtins.map
              (flags: {
                name = "tests-(${flags})";
                value = {
                  enable = true;
                  name = "Unit and integration tests (${flags})";
                  entry = "${pkgs.cargo}/bin/cargo test --workspace ${flags}";
                  pass_filenames = false;
                };
              })
              [
                ""
                "--release"
              ]
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
                  settings.extraArgs = "--keep-going";
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

      packages = forEachSupportedSystem (
        { pkgs, ... }:
        {
          default = pkgs.rustPlatform.buildRustPackage {
            name = "cheers";
            src = ./.;
            cargoBuildFlags = [ "-p=cheers_cli" ];

            cargoHash = "sha256-UoiCdeQ6sveUnKhL9JnLBqO7ysaeuXw9h58V2pSF5Og=";

            doCheck = false;
          };
        }
      );
    };
}
