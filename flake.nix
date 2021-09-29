{
  description = "ntp-proxy";

  outputs = { self, nixpkgs }: let
    version = self.shortRev or (toString self.lastModifiedDate);
    overlay = final: prev: {
      ntp-proxy = final.callPackage (
        { rustPlatform }: rustPlatform.buildRustPackage {
          pname = "ntp-proxy";
          inherit version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
        }
      ) {};

      ntp-proxy-pkg = final.callPackage (
        { ntp-proxy, zstd }: pkgs.runCommand "ntp-proxy-pkg" {
          nativeBuildInputs = [ zstd ];
        } ''
          mkdir -p usr/bin $out
          cp ${ntp-proxy}/bin/ntp_proxy usr/bin
          tar --zstd -cf $out/ntp-proxy-x86_64-${ntp-proxy.version}.pkg usr
        ''
      ) {};
    };
    pkgs = import nixpkgs {
      system = "x86_64-linux";
      crossSystem = {
        isStatic = true;
        config = "x86_64-unknown-linux-musl";
      };
      overlays = [ overlay ];
    };
  in {
    inherit overlay;
    packages.x86_64-linux = {
      inherit (pkgs) ntp-proxy ntp-proxy-pkg;
    };
    defaultPackage.x86_64-linux = self.packages.x86_64-linux.ntp-proxy-pkg;
  };
}
