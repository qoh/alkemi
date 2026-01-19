{
  pkgs ? import <nixpkgs> { },
}:
let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { };
  rustToolchain = fenix.stable;
  libPath =
    with pkgs;
    lib.makeLibraryPath [
      udev
      alsa-lib
      vulkan-loader
      libGL
      libxkbcommon
      wayland
      # xorg.libX11
      # xorg.libXcursor
      # xorg.libXi
      # xorg.libXrandr

      openssl # XXX: maybe unnecessary
    ];
in
pkgs.mkShell {
  strictDeps = true;

  packages = [
    (rustToolchain.withComponents [
      "cargo"
      "clippy"
      "rust-src"
      "rustc"
      "rustfmt"
      "rust-analyzer"
    ])
    # rustToolchain.rust-analyzer
    # inputs'.fenix.packages.rust-analyzer-nightly
  ];

  nativeBuildInputs = [
    pkgs.pkg-config
  ];

  buildInputs = [
    pkgs.openssl

    # Bevy
    pkgs.udev
    pkgs.alsa-lib
    pkgs.wayland
  ];

  RUST_SRC_PATH = "${rustToolchain.rust-src}/lib/rustlib/src/rust/";
  LD_LIBRARY_PATH = libPath;
}
