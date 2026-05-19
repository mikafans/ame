{
  description = "ame — question collector + exam platform devShell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # postgres client only — engine runs in docker
            postgresql_18

            # docker (CLI; daemon is OrbStack / Docker Desktop / Colima on the host)
            docker-client
            docker-compose

            # sql + format tooling
            sqlfluff

            # misc
            git
            gh
            gnumake
            jq
            ripgrep
            fd
            fzf
          ];

          shellHook = ''
            echo "ame devShell — language runtimes via mise (rust $(mise current rust 2>/dev/null || echo '?'), bun $(mise current bun 2>/dev/null || echo '?'))"
          '';
        };
      });
}
