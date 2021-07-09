{ containerTag ? "latest"
, prefixName ? ""
}:
let
  sources = import ./sources.nix;
  pkgs = import sources.nixpkgs {};
  ergvein-rusty = import ../default.nix;

  baseImage = pkgs.dockerTools.pullImage {
      imageName = "alpine";
      imageDigest = "sha256:e1871801d30885a610511c867de0d6baca7ed4e6a2573d506bbec7fd3b03873f";
      sha256 = "05wcg38vsygjzf59cspfbb7cq98c7x18kz2yym6rbdgx960a0kyq";
    };

  # As we place all executables in single derivation the derivation takes them
  # from it and allows us to make thin containers for each one.
  takeOnly = name: path: pkgs.runCommandNoCC "only-${name}" {} ''
    mkdir -p $out
    cp ${path} $out/${name}
  '';
  takeFolder = name: path: innerPath: pkgs.runCommandNoCC "folder-${name}" {} ''
    mkdir -p $out/${innerPath}
    cp -r ${path}/* $out/${innerPath}
  '';

  mkDockerImage = name: cnts: pkgs.dockerTools.buildImage {
    name = "${prefixName}${name}";
    fromImage = baseImage;
    tag = containerTag;
    contents = cnts;
    config = {
      Entrypoint = [
        "/ergvein-rusty"
      ];
    };
  };

  ergvein-rusty-container = mkDockerImage "ergvein-rusty" [
    (takeOnly "ergvein-rusty" "${ergvein-rusty}/bin/ergvein-rusty")
  ];
in { inherit
  ergvein-rusty-container
  ;
}
