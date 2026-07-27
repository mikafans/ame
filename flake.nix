{
  description = "AME development environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          # Language runtimes and CLIs come from the devShell (nix-first). No mise.
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.bun
            pkgs.uv
            pkgs.python314
            pkgs.sqlx-cli
            # podman-compose drives the podman CLI directly (over the machine's
            # SSH connection) — no docker-compose provider, no DOCKER_HOST socket.
            pkgs.podman-compose
          ];

          env = {
            RUST_BACKTRACE = "1";
            # uv resolves deps and runs uvx tools, but the interpreter is the
            # nix-provided python314 — one source of truth, no downloaded CPython.
            UV_PYTHON_PREFERENCE = "only-system";
          };
        };
      });
    };
}
