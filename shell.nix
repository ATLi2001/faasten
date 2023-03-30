{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  buildInputs = [ rustup rustfmt protobuf pkg-config openssl unzip cmake ];
}
