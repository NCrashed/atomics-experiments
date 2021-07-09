let
sources = import ./nix/sources.nix;
pkgs = import sources.nixpkgs {};
in with pkgs;
let merged-openssl = symlinkJoin { name = "merged-openssl"; paths = [ openssl.out openssl.dev ]; };
in stdenv.mkDerivation rec {
  name = "rust-env";
  env = buildEnv { name = name; paths = buildInputs; };

  buildInputs = [
    rustup
    clang
    llvmPackages_11.libclang
    openssl
    valgrind
    llvm_11
    #docker
    docker-compose
    btcdeb
    miniscript
  ];
  shellHook = ''
  export LIBCLANG_PATH="${llvmPackages.libclang}/lib"
  export OPENSSL_DIR="${merged-openssl}"
  '';
}
