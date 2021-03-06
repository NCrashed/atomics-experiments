{ config, lib, pkgs, ... }:
with lib;  # use the functions from lib, such as mkIf
let
  # the values of the options set for the service by the user of the service
  cfg = config.services.ergvein;
in {
  ##### Depedendant services
  imports = [
    ./bitcoin.nix
    ./ergvein-rusty.nix
  ];

  ##### interface. here we define the options that users of our service can specify
  options = {
    services.ergvein = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Whether to enable ergvein indexer system by default.
        '';
      };
      testnet = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Start in testnet mode. Uses different data dir.
        '';
      };
      externalAddress = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Which IP and port is assigned to the node as external. Example: 192.168.0.1:8667
        '';
      };
      metrics = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Start prometheus and grafana with local metrics for the indexer.
        '';
      };
    };
  };

  ##### implementation
  config = mkIf cfg.enable { # only apply the following settings if enabled
    nixpkgs.overlays = [
      (import ../overlay.nix)
    ];
    services = {
      bitcoin = {
        enable = true;
        testnet = cfg.testnet;
        nodePort = 8332;
        package = with pkgs; pkgs.callPackage ../pkgs/bitcoin-node.nix { withGui = false; };
      };
      ergvein-rusty = {
        enable = true;
        package = pkgs.ergvein-rusty;
        externalAddress = cfg.externalAddress;
        /* testnet = cfg.testnet; */
        metrics = if cfg.metrics then {
          host = "127.0.0.1";
          port = 9667;
        } else null;
      };
      grafana = {
        enable = cfg.metrics;
        provision = {
          enable = true;
          datasources = [
            {
              name = "Prometheus";
              type = "prometheus";
              isDefault = true;
              url = "http://127.0.0.1:9090";
            }
          ];
          dashboards = [
            {
              options.path = ./dashboards;
            }
          ];
        };
      };
      prometheus = {
        enable = cfg.metrics;
        scrapeConfigs = [
          {
            job_name = "node";
            scrape_interval = "10s";
            metrics_path = "/";
            static_configs = [
              {
                targets = [
                  "127.0.0.1:9667"
                ];
                labels = {
                  alias = "indexer";
                };
              }
            ];
          }
        ];
      };
    };
  };
}
