# NixOS module: the MFTR game server as a systemd service (docs/hosting.md, "NixOS").
#
#   services.mftr.enable = true;          # every mode on UDP 7777, like the Docker image
#   services.mftr.openFirewall = true;
#
# One server plays whatever its players pick (docs/design/13-server-instances.md): ARAM,
# ARAM: Mayhem, ARAM: Hyper and Classic, each Versus or Co-op vs bots, as many matches at once
# as players fill (up to `maxMatches`). Each map file in `maps` adds Classic on that map:
#
#   services.mftr = {
#     enable = true;
#     openFirewall = true;
#     maps = [ ./maps/twin-rivers.mftrmap ];
#     replay = true;
#   };
#
# The server keeps its key and replays in /var/lib/mftr. Clients pin the key, so it survives
# upgrades and rebuilds; back that directory up with the machine.
{ config, lib, pkgs, ... }:

let
  inherit (lib) mkOption mkEnableOption types;
  cfg = config.services.mftr;
  dir = "/var/lib/mftr";
  bind = if lib.hasInfix ":" cfg.address then "[${cfg.address}]:${toString cfg.port}" else "${cfg.address}:${toString cfg.port}";
  args = [
    (lib.getExe' cfg.package "mftr-server")
    "--bind" bind
    "--key" cfg.keyFile
    "--seed" (toString cfg.seed)
    "--modes" (lib.concatStringsSep "," cfg.modes)
    "--max-matches" (toString cfg.maxMatches)
  ]
  ++ lib.concatMap (m: [ "--map" "${m}" ]) cfg.maps
  ++ lib.optionals cfg.replay [ "--replay" "${dir}/replays" ]
  ++ cfg.extraArgs;
in
{
  imports = [
    (lib.mkRemovedOptionModule [ "services" "mftr" "servers" ] ''
      One MFTR server now plays every mode on one port (players pick what to play; see
      docs/hosting.md). Use services.mftr.{port,modes,maps,replay}. To keep the key players
      pinned, point services.mftr.keyFile at the old server's key, for example
      "/var/lib/mftr/aram/server.key".
    '')
  ];

  options.services.mftr = {
    enable = mkEnableOption "the MFTR game server";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = lib.literalExpression "the mftr flake's mftr-server package";
      description = "The package with `mftr-server` and `mftr-tools`.";
    };

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

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open the UDP port in the firewall.";
    };

    modes = mkOption {
      type = types.nonEmptyListOf (types.enum [ "aram" "mayhem" "hyper" "classic" ]);
      default = [ "aram" "mayhem" "hyper" "classic" ];
      description = ''
        What players can pick, each Versus or Co-op vs bots. `aram`: The Bridge, a full match
        with a winner. `mayhem`: ARAM with augment drafts. `hyper`: Mayhem under Hyper rules.
        `classic`: Classic 5v5 on Crossroads (and on each of `maps`).
      '';
    };

    maps = mkOption {
      type = types.listOf types.path;
      default = [ ];
      example = lib.literalExpression "[ ./maps/twin-rivers.mftrmap ]";
      description = ''
        Map files exported from the map editor: each adds Classic on that map. Players download
        a map as they join. The server refuses to start if a map breaks a rule.
      '';
    };

    maxMatches = mkOption {
      type = types.ints.positive;
      default = 32;
      description = ''
        Matches running at once, at most. Past it, players who'd need a new match are told the
        server is busy (their client keeps asking).
      '';
    };

    seed = mkOption {
      type = types.ints.unsigned;
      default = 1;
      description = "Seeds everything random in the matches (each match draws its own from it).";
    };

    keyFile = mkOption {
      type = types.str;
      default = "${dir}/server.key";
      example = "/var/lib/mftr/aram/server.key";
      description = ''
        The server's key, created on first start. Clients pin it: to keep an older server's key,
        point this at it.
      '';
    };

    replay = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Record every match into ${dir}/replays (rewritten every minute and when a match ends;
        check one with `mftr-tools replay`).
      '';
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "More `mftr-server` options.";
    };
  };

  config = lib.mkIf cfg.enable {
    # `mftr-tools replay`, `mftr-server --fingerprint` and the bot client on the command line.
    environment.systemPackages = [ cfg.package ];

    systemd.services.mftr = {
      description = "MFTR game server";
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
        StateDirectory = "mftr";
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

    networking.firewall.allowedUDPPorts = lib.optional cfg.openFirewall cfg.port;
  };
}
