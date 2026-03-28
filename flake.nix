{
  inputs = {
    # Needed due to vendoring datastar
    self.submodules = true;

    nixpkgs.url = "nixpkgs/nixos-unstable";

    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
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
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          let
            overlays = [ ];
            pkgs = import inputs.nixpkgs { inherit overlays system; };
            pre-commit-hooks = inputs.pre-commit-hooks.lib.${system}.run;
          in
          f {
            inherit pkgs pre-commit-hooks;
            inherit (inputs.self.checks.${system}) pre-commit-check;
          }
        );
    in
    {
      devShells = forEachSupportedSystem (
        {
          pkgs,
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

              cargo
              rustc
              cargo-deny
              cargo-machete
              cargo-nextest
              rust-analyzer

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
          testHooks = {
            nextest = {
              enable = true;
              raw.priority = 41;
              name = "nextest";
              entry = "${pkgs.cargo}/bin/cargo nextest run --workspace";
              pass_filenames = false;
              extraPackages = [ pkgs.cargo-nextest ];
            };
            nextest-release = {
              enable = true;
              raw.priority = 42;
              name = "nextest (--release)";
              entry = "${pkgs.cargo}/bin/cargo nextest run --workspace --release";
              pass_filenames = false;
              extraPackages = [ pkgs.cargo-nextest ];
            };
          };
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
              package = pkgs.prek;
              inherit default_stages;
              excludes = [
                "(^|/)\\.direnv/"
              ];
              hooks = testHooks // {
                nixfmt = {
                  enable = true;
                  raw.priority = 0;
                };
                cargo-machete = {
                  enable = true;
                  raw.priority = 10;
                  name = "cargo-machete";
                  entry = ''
                    sh -eu -c '${pkgs.cargo}/bin/cargo metadata --no-deps --format-version 1 \
                      | ${pkgs.jq}/bin/jq -r ".packages[] | select(.name != \"workspace-hack\") | .manifest_path" \
                      | while IFS= read -r manifest; do
                          ${pkgs.cargo-machete}/bin/cargo-machete --with-metadata --fix "$manifest";
                        done'
                  '';
                  always_run = true;
                  pass_filenames = false;
                };
                taplo = {
                  enable = true;
                  raw.priority = 12;
                };
                typos = {
                  enable = true;
                  raw.priority = 20;
                  stages = default_stages ++ [ "commit-msg" ];
                };
                actionlint = {
                  enable = true;
                  raw.priority = 30;
                };
                check-added-large-files = {
                  enable = true;
                  raw.priority = 30;
                };
                check-case-conflicts = {
                  enable = true;
                  raw.priority = 30;
                  stages = [
                    "pre-commit"
                    "pre-push"
                    "manual"
                  ];
                };
                check-merge-conflicts = {
                  enable = true;
                  raw.priority = 30;
                  stages = [
                    "pre-commit"
                    "pre-push"
                    "manual"
                  ];
                };
                cargo-deny = {
                  enable = true;
                  raw.priority = 30;
                  name = "cargo-deny";
                  entry = "${pkgs.cargo-deny}/bin/cargo-deny check";
                  files = "(^|/)(Cargo\\.toml|Cargo\\.lock|deny\\.toml)$";
                  pass_filenames = false;
                };
                deadnix = {
                  enable = true;
                  raw.priority = 30;
                };
                gitleaks = {
                  enable = true;
                  raw.priority = 30;
                  name = "gitleaks";
                  package = pkgs.gitleaks;
                  entry = "${pkgs.gitleaks}/bin/gitleaks git --staged --no-banner --verbose";
                  always_run = true;
                  pass_filenames = false;
                  stages = [ "pre-commit" ];
                };
                statix = {
                  enable = true;
                  raw.priority = 30;
                  settings.ignore = [
                    ".direnv/**"
                  ];
                };
                clippy = {
                  enable = true;
                  raw.priority = 40;
                  settings = {
                    allFeatures = true;
                    denyWarnings = true;
                    extraArgs = "--keep-going";
                  };
                };
                rustfmt = {
                  enable = true;
                  raw.priority = 0;
                };
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
            name = "cargo-cheers";
            src = ./.;
            cargoBuildFlags = [ "-p=cargo-cheers" ];

            cargoHash = "sha256-MGqJKbrqgTM+qbd0gMmFayUW8YnoQ4qgRDj8bEt6kxE=";

            doCheck = false;
          };
        }
      );
    };
}
