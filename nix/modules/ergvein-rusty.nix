{ config, lib, ... }:
with lib;  # use the functions from lib, such as mkIf
let
  pkgs = import <nixpkgs> { };
  # the values of the options set for the service by the user of the service
  cfg = config.services.ergvein-rusty;

  metricsType = {
    options = {
      host = mkOption {
        type = types.str;
        example = "127.0.0.1";
        description = ''
          Hostname of metrics client server.
        '';
      };
      port = mkOption {
        type = types.int;
        example = 9667;
        description = ''
          Port of metrics client.
        '';
      };
    };
  };

in {
  ##### interface. here we define the options that users of our service can specify
  options = {
    # the options for our service will be located under services.ergvein-rusty
    services.ergvein-rusty = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Whether to enable ergvein indexer node by default.
        '';
      };
      package = mkOption {
        type = types.package;
        default = pkgs.ergvein-rusty;
        description = ''
          Which package to use with the service.
        '';
      };
      externalAddress = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Which IP and port is assigned to the node as external. Example: 192.168.0.1:8667
        '';
      };
      port = mkOption {
        type = types.int;
        default = 8667;
        description = ''
          Which port the indexer listen to TCP protocol connections.
        '';
      };
      host = mkOption {
        type = types.str;
        default = "0.0.0.0";
        description = ''
          Which hostname is binded to the node.
        '';
      };
      statePath = mkOption {
        type = types.str;
        default = "/var/lib/ergvein";
        description = ''
          Path to filters database on filesystem.
        '';
      };

      btcNode = mkOption {
        type = types.str;
        default = "127.0.0.1:8333";
        description = ''
          Host and port where BTC node is located.
        '';
      };

      blockBatch = mkOption {
        type = types.int;
        default = 100;
        description = ''
          Amount of blocks to process in parallel while syncing
        '';
      };
      flushPeriod = mkOption {
        type = types.int;
        default = 15000;
        description = ''
          Flush cache to disk every given amount of blocks
        '';
      };
      forkDepth = mkOption {
        type = types.int;
        default = 100;
        description = ''
          Maximum reorganizatrion depth in blockchain
        '';
      };
      maxCache = mkOption {
        type = types.int;
        default = 17000000;
        description = ''
          Maximum size of cache in coins amount, limits memory for cache
        '';
      };

      metrics = mkOption {
        type = types.nullOr (types.submodule metricsType);
        default = null;
        description = ''
          Start metrics client inside the indexer if not null.
        '';
      };
    };
  };

  ##### implementation
  config = mkIf cfg.enable { # only apply the following settings if enabled

    # Create systemd service
    systemd.services.ergvein-rusty = {
      enable = true;
      description = "Ergvein indexer node";
      after = ["network.target"];
      wants = ["network.target"];
      script = ''
        ${cfg.package}/bin/ergvein-rusty --bitcoin ${cfg.btcNode} \
            --block-batch ${builtins.toString cfg.blockBatch} \
            --data ${cfg.statePath} \
            --flush-period ${builtins.toString cfg.flushPeriod} \
            --fork-depth ${builtins.toString cfg.forkDepth} \
            --host ${cfg.host} \
            --port ${builtins.toString cfg.port} \
            --max-cache ${builtins.toString cfg.maxCache} \
            --metrics-host ${cfg.metrics.host} \
            --metrics-port ${builtins.toString cfg.metrics.port}
      '';
      serviceConfig = {
          Restart = "always";
          RestartSec = 30;
          User = "root";
          LimitNOFILE = 65536;
        };
      wantedBy = ["multi-user.target"];
    };
    # Init folder for bitcoin data
    system.activationScripts = {
      int-ergvein-rusty = {
        text = ''
          if [ ! -d "${cfg.statePath}" ]; then
            mkdir -p ${cfg.statePath}
          fi
        '';
        deps = [];
      };
    };
  };
}
