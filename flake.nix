{
  description = "A TUI for browsing and previewing Ghostty terminal themes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Platform-specific native build inputs for reqwest's native-tls
        darwinBuildInputs = with pkgs; [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        linuxBuildInputs = with pkgs; [
          openssl
        ];

        platformBuildInputs =
          if pkgs.stdenv.hostPlatform.isDarwin then darwinBuildInputs
          else linuxBuildInputs;

        platformNativeBuildInputs =
          if pkgs.stdenv.hostPlatform.isLinux then [ pkgs.pkg-config ]
          else [ ];
      in
      {
        packages = {
          ghostty-styles = pkgs.rustPlatform.buildRustPackage {
            pname = "ghostty-styles";
            version = "0.1.0";

            src = pkgs.lib.cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = platformNativeBuildInputs;
            buildInputs = platformBuildInputs;

            meta = with pkgs.lib; {
              description = "A TUI for browsing and previewing Ghostty terminal themes";
              homepage = "https://github.com/mcfearsome/ghostty.styles.tui";
              license = licenses.mit;
              maintainers = [ ];
              mainProgram = "ghostty-styles";
            };
          };

          default = self.packages.${system}.ghostty-styles;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.ghostty-styles ];

          packages = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      }
    );
}
