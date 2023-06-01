{
  description = "A very basic flake";

  inputs.nixpkgs.url = "nixpkgs/nixpkgs-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
    in {

      packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;
      packages.x86_64-linux.default = self.packages.x86_64-linux.hello;

      devShells.${system}.default = pkgs.mkShell {
        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

        packages = with pkgs; [

          # (rust-bin.beta.latest.default.override {
          #   extensions = [ "rust-src" "rust-std" "rustc"  ];
          #   targets = [ "arm-unknown-linux-gnueabihf" "thumbv6m-none-eabi" ];
          # })
          (rust-bin.selectLatestNightlyWith (toolchain:
            toolchain.default.override {
              extensions = [ "rust-src" "rust-std" "rustc" ];
              targets = [ "arm-unknown-linux-gnueabihf" "thumbv6m-none-eabi" ];
            }))

          pkg-config
          rust-analyzer

          probe-run
          probe-rs-cli
        ];
      };

    };
}
