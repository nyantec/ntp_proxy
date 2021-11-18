{
  description = "ntp-proxy";

  outputs = { self, nixpkgs }: let
    overlay = final: prev: {
      ntp-proxy = final.callPackage (
        { rustPlatform }: rustPlatform.buildRustPackage {
          pname = "ntp-proxy";
          version = self.shortRev or "dirty-${toString self.lastModifiedDate}";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
        }
      ) {};
    };
  in {
    inherit overlay;
    packages.x86_64-linux = import nixpkgs {
      system = "x86_64-linux";
      overlays = [ overlay ];
    };
    defaultPackage.x86_64-linux = self.packages.x86_64-linux.ntp-proxy;
  };
}
