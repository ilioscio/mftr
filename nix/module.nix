# NixOS module: MFTR game servers as systemd services (docs/hosting.md, "NixOS").
#
#   services.mftr.enable = true;          # one ARAM server on UDP 7777, like the Docker image
#   services.mftr.openFirewall = true;
#
# or several, one service each (`mftr-<name>.service`):
#
#   services.mftr.servers = {
#     aram = { port = 7777; replay = true; };
#     duel = { port = 7778; scenario = "duel"; maxPlayers = 4; };
#   };
#
# Each server keeps its key and replay in /var/lib/mftr/<name>. Clients pin the key, so it
# survives upgrades and rebuilds; back that directory up with the machine.
{ config, lib, pkgs, ... }:

let
  inherit (lib) mkOption mkEnableOption types;
  cfg = config.services.mftr;
  enabled = lib.filterAttrs (_: s: s.enable) cfg.servers;

  server = { name, config, ... }: {
    options = {
      enable = mkEnableOption "this MFTR server" // { default = true; };

      address = mkOption {
        type = types.str;
        default = "0.0.0.0";
        example = "::";
        description = ''
          Address to listen on. `::` listens on IPv6 and, on Linux's default settings, IPv4 too.
        '';
      };

      port = mkOption {
        type = types.port;
        default = 7777;
        description = "UDP port.";
      };

      scenario = mkOption {
        type = types.enum [ "aram" "duel" "minions" "dodge" "empty" ];
        default = "aram";
        description = ''
          `aram`: The Bridge, a full match with a winner. `duel`: the Duel Sandbox. `minions`,
          `dodge`, `empty`: test grounds (`dodge` is the blind playtest's dodge rig).
        '';
      };

      bots = mkOption {
        type = types.ints.u8;
        default = if config.scenario == "aram" then 10 else 0;
        defaultText = lib.literalExpression ''if scenario == "aram" then 10 else 0'';
        description = ''
          Server bots. They count toward `maxPlayers`, and with `lobby` a joining human takes a
          bot's place.
        '';
      };

      lobby = mkOption {
        type = types.bool;
        default = config.scenario == "aram";
        defaultText = lib.literalExpression ''scenario == "aram"'';
        description = "Champion select before each match (ARAM all-random with rerolls).";
      };

      maxPlayers = mkOption {
        type = types.ints.between 1 255;
        default = 10;
        description = "Players (humans and bots) per game. Up to 8 spectators come on top.";
      };

      seed = mkOption {
        type = types.ints.unsigned;
        default = 1;
        description = "Seeds everything random in the match.";
      };

      replay = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Record the session to /var/lib/mftr/${name}/session.replay (rewritten every minute and
          at each match end; check it with `mftr-tools replay`).
        '';
      };

      openFirewall = mkOption {
        type = types.bool;
        default = cfg.openFirewall;
        defaultText = lib.literalExpression "config.services.mftr.openFirewall";
        description = "Open this server's UDP port in the firewall.";
      };

      extraArgs = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "More `mftr-server` options.";
      };
    };
  };

  bind = s: if lib.hasInfix ":" s.address then "[${s.address}]:${toString s.port}" else "${s.address}:${toString s.port}";

  service = name: s:
    let
      dir = "/var/lib/mftr/${name}";
      args = [
        (lib.getExe' cfg.package "mftr-server")
        "--bind" (bind s)
        "--key" "${dir}/server.key"
        "--scenario" s.scenario
        "--bots" (toString s.bots)
        "--max-players" (toString s.maxPlayers)
        "--seed" (toString s.seed)
      ]
      ++ lib.optional s.lobby "--lobby"
      ++ lib.optionals s.replay [ "--replay" "${dir}/session.replay" ]
      ++ s.extraArgs;
    in
    lib.nameValuePair "mftr-${name}" {
      description = "MFTR game server (${name})";
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      # A new build (`nix flake update mftr` + a rebuild) restarts the server.
      restartIfChanged = true;
      serviceConfig = {
        ExecStart = lib.escapeShellArgs args;
        Restart = "on-failure";
        RestartSec = 2;

        DynamicUser = true;
        StateDirectory = "mftr/${name}";
        WorkingDirectory = dir;
        UMask = "0077";

        # A UDP socket and its own directory is all it needs.
        CapabilityBoundingSet = "";
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectHome = true;
        ProtectSystem = "strict";
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" ];
      };
    };
in
{
  options.services.mftr = {
    enable = mkEnableOption "MFTR game servers";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = lib.literalExpression "the mftr flake's mftr-server package";
      description = "The package with `mftr-server` and `mftr-tools`.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open every server's UDP port in the firewall (each server can override it).";
    };

    servers = mkOption {
      type = types.attrsOf (types.submodule server);
      default = { aram = { }; };
      defaultText = lib.literalExpression "{ aram = { }; }";
      example = lib.literalExpression ''
        {
          aram = { port = 7777; replay = true; };
          duel = { port = 7778; scenario = "duel"; maxPlayers = 4; };
        }
      '';
      description = ''
        The servers to run, one systemd service `mftr-<name>` each. The default is one ARAM
        server with champion select and 10 bots on UDP 7777.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions =
      let
        ports = lib.mapAttrsToList (_: s: s.port) enabled;
      in
      [
        {
          assertion = lib.length ports == lib.length (lib.unique ports);
          message = "services.mftr.servers: every enabled server needs its own port.";
        }
      ];

    # `mftr-tools replay`, `mftr-server --fingerprint` and the bot client on the command line.
    environment.systemPackages = [ cfg.package ];

    systemd.services = lib.mapAttrs' service enabled;

    networking.firewall.allowedUDPPorts = lib.mapAttrsToList (_: s: s.port) (lib.filterAttrs (_: s: s.openFirewall) enabled);
  };
}
