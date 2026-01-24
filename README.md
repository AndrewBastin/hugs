# Hugs (っ◕‿◕)っ
Hugs is a very simplicitic static site generator specifically focused on making simple personal websites.


## How to install
Hugs is currently not published anywhere, so you will have to have the [Rust toolchain](https://rust-lang.org/tools/install/) and then compile and install against the source code using [`cargo install`](https://doc.rust-lang.org/cargo/commands/cargo-install.html).

```sh
git clone https://github.com/AndrewBastin/hugs
cd hugs
cargo install --path .
```

### Nix
If you use Nix to set up your website dev shell, you can something like the following with Nix flakes.
```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    hugs.url = "github:AndrewBastin/hugs";
  };

  outputs = { self, nixpkgs, hugs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        hugs = hugs.packages.${system}.default;
      in
        {
          # default package that builds your site
          packages.default = pkgs.runCommand "my-site" {
              nativeBuildInputs = [ hugs ];
          } ''
            hugs build ${./.} -o $out
          '';

          # development shell
          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              # Exposes the `hugs` command in your dev shell
              hugs
            ];
          };
        }
    );
}
```

## How to use ?
Hugs is self documenting. The binary contains all the documentation about it. Run `hugs --help` to see your options.
