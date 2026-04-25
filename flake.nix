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
              chromium
              chromedriver

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
          readme-sync-check = pkgs.writeShellApplication {
            name = "readme-sync-check";
            runtimeInputs = [ pkgs.python3 ];
            text = ''
              python3 ${./scripts/readme-sync-check.py}
            '';
          };
          readme-doctest-check = pkgs.writeShellApplication {
            name = "readme-doctest-check";
            runtimeInputs = with pkgs; [
              cargo
              rustc
              chromium
              chromedriver
            ];
            text = ''
              cargo test --doc -p cheers --all-features
            '';
          };
          skill-rust-block-fmt = pkgs.writeShellApplication {
            name = "skill-rust-block-fmt";
            runtimeInputs = [
              inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.default
              pkgs.python3
              pkgs.rustfmt
            ];
            text = ''
              python3 ${./scripts/format-skill-rust-blocks.py} "$@"
            '';
          };
        in
        {
          default-package-build = inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.default;

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
              hooks = {
                rustfmt = {
                  enable = true;
                  raw.priority = 0;
                };
                skill-rust-block-fmt = {
                  enable = true;
                  name = "format Rust blocks in skill files";
                  entry = "${skill-rust-block-fmt}/bin/skill-rust-block-fmt";
                  files = "^skills/.*\\.md$";
                  raw.priority = 0;
                };
                nixfmt = {
                  enable = true;
                  raw.priority = 0;
                };
                shellcheck = {
                  enable = true;
                  raw.priority = 0;
                  excludes = [ "(^|/)\\.envrc$" ];
                };
                commitlint =
                  let
                    config = pkgs.writeText "commitlint.config.json" (
                      builtins.toJSON {
                        rules = {
                          "body-leading-blank" = [
                            1
                            "always"
                          ];
                          "body-max-line-length" = [
                            2
                            "always"
                            100
                          ];
                          "footer-leading-blank" = [
                            1
                            "always"
                          ];
                          "footer-max-line-length" = [
                            2
                            "always"
                            100
                          ];
                          "header-max-length" = [
                            2
                            "always"
                            100
                          ];
                          "header-trim" = [
                            2
                            "always"
                          ];
                          "subject-case" = [
                            2
                            "never"
                            [
                              "sentence-case"
                              "start-case"
                              "pascal-case"
                              "upper-case"
                            ]
                          ];
                          "subject-empty" = [
                            2
                            "never"
                          ];
                          "subject-full-stop" = [
                            2
                            "never"
                            "."
                          ];
                          "type-case" = [
                            2
                            "always"
                            "lower-case"
                          ];
                          "type-empty" = [
                            2
                            "never"
                          ];
                          "type-enum" = [
                            2
                            "always"
                            [
                              "build"
                              "chore"
                              "ci"
                              "docs"
                              "feat"
                              "fix"
                              "perf"
                              "refactor"
                              "revert"
                              "style"
                              "test"
                            ]
                          ];
                        };
                      }
                    );
                  in
                  {
                    enable = true;
                    name = "commitlint";
                    entry = "${pkgs.commitlint}/bin/commitlint --from origin/main --to HEAD --config ${config}";
                    raw.priority = 0;
                    pass_filenames = false;
                    always_run = true;
                  };
                readme-sync = {
                  enable = true;
                  raw.priority = 30;
                  name = "README example sync";
                  entry = "${readme-sync-check}/bin/readme-sync-check";
                  pass_filenames = false;
                  always_run = true;
                };
                cargo-machete = {
                  enable = true;
                  raw.priority = 30;
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
                  raw.priority = 30;
                };
                typos = {
                  enable = true;
                  raw.priority = 30;
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
                  stages = [
                    "pre-commit"
                    "manual"
                  ];
                };
                statix = {
                  enable = true;
                  raw.priority = 30;
                  settings.ignore = [
                    ".direnv/**"
                  ];
                };
                biome = {
                  enable = true;
                  excludes = [
                    "(^|/)vendor/"
                  ];
                  raw.types_or = [
                    "ts"
                    "json"
                    "css"
                  ];
                  raw.priority = 30;
                  settings.write = true;
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
                nextest = {
                  enable = true;
                  raw.priority = 41;
                  name = "nextest";
                  entry = "${pkgs.cargo}/bin/cargo nextest run --workspace";
                  pass_filenames = false;
                  extraPackages = with pkgs; [
                    bash
                    cargo-nextest
                    chromium
                    chromedriver
                    coreutils
                    util-linux
                  ];
                };
                readme-doctests = {
                  enable = true;
                  raw.priority = 41;
                  name = "README doctests";
                  entry = "${readme-doctest-check}/bin/readme-doctest-check";
                  pass_filenames = false;
                  always_run = true;
                };
                nextest-release = {
                  enable = true;
                  raw.priority = 44;
                  name = "nextest (--release)";
                  entry = "${pkgs.cargo}/bin/cargo nextest run --workspace --release";
                  pass_filenames = false;
                  extraPackages = with pkgs; [
                    bash
                    cargo-nextest
                    chromium
                    chromedriver
                    coreutils
                    util-linux
                  ];
                };
              };
              settings = {
                rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
                  lockFile = ./Cargo.lock;
                  outputHashes = {
                    "selenium-manager-0.4.36" = "sha256-MQX14lk6X/ShAnqRe6GGkC+SnOO2J2x43wFi2uB7uRc=";
                  };
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

            cargoHash = "sha256-XZHqZVOzOsFeDQzS2+Y9CXkpoxHXUStbpLCNMLSgR30=";

            doCheck = false;
          };
        }
      );
    };
}
