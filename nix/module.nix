# NixOS module: MFTR game servers as systemd services (docs/hosting.md, "NixOS").
#
#   services.mftr.enable = true;          # every mode on UDP 7777, like the Docker image
#   services.mftr.openFirewall = true;
#
# The main server plays whatever its players pick (docs/design/13-server-instances.md): ARAM,
# ARAM: Mayhem, ARAM: Hyper and Classic, each Versus or Co-op vs bots, as many matches at once
# as players fill (up to `maxMatches`). Each map file in `maps` adds Classic on that map.
# Sandboxes (the Duel Sandbox, the dodge rig, ...) run as servers of their own:
#
#   services.mftr = {
#     enable = true;
#     openFirewall = true;
#     modes = [ "aram" "mayhem" "classic" "hyper-coop" ];
#     maps = [ ./maps/twin-rivers.mftrmap ];
#     replay = true;
#     sandboxes.duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };
#   };
#
# Each server keeps its key (and replays) in /var/lib/mftr/<name>: the main one in
# /var/lib/mftr/server, a sandbox in /var/lib/mftr/<its name>. Clients pin the keys, so they
# survive upgrades and rebuilds; back that directory up with the machine.
{ config, lib, pkgs, ... }:

let
  inherit (lib) mkOption mkEnableOption types;
  cfg = config.services.mftr;
  enabledSandboxes = lib.filterAttrs (_: s: s.enable) cfg.sandboxes;
  fullModes = [ "aram" "mayhem" "hyper" "classic" ];
  bind = address: port: if lib.hasInfix ":" address then "[${address}]:${toString port}" else "${address}:${toString port}";

  # A server's systemd service: unprivileged, sandboxed, its state in /var/lib/mftr/<name>.
  service = { name, description, args }: {
    inherit description;
    wantedBy = [ "multi-user.target" ];
    wants = [ "network-online.target" ];
    after = [ "network-online.target" ];
    # A new build (`nix flake update mftr` + a rebuild) restarts the server.
    restartIfChanged = true;
    serviceConfig = {
      ExecStart = lib.escapeShellArgs ([ (lib.getExe' cfg.package "mftr-server") ] ++ args);
      Restart = "on-failure";
      RestartSec = 2;

      DynamicUser = true;
      StateDirectory = "mftr/${name}";
      WorkingDirectory = "/var/lib/mftr/${name}";
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

  # Before the main server's first start, take over an older server's key (players pinned
  # it): from `adoptKeyFrom`'s directory, or from where 0.11.0 kept the main server's key. It
  # runs as its own unsandboxed service: inside the server's service, even a `+` command only
  # sees its own state directory under /var/lib/private.
  adoptKey = pkgs.writeShellScript "mftr-adopt-key" ''
    set -eu
    dest=/var/lib/private/mftr/server
    [ -e "$dest/server.key" ] && exit 0
    for old in ${lib.escapeShellArgs (lib.optional (cfg.adoptKeyFrom != null) "/var/lib/private/mftr/${cfg.adoptKeyFrom}/server.key" ++ [ "/var/lib/private/mftr/server.key" ])}; do
      if [ -e "$old" ]; then
        # A directory owned by root here is handed to the server's user when it starts.
        ${pkgs.coreutils}/bin/mkdir -p "$dest"
        ${pkgs.coreutils}/bin/cp "$old" "$dest/server.key"
        ${pkgs.coreutils}/bin/chmod 600 "$dest/server.key"
        ${pkgs.coreutils}/bin/chown --reference="$dest" "$dest/server.key"
        echo "mftr: adopted the server key from $old"
        exit 0
      fi
    done
  '';

  sandbox = { name, config, ... }: {
    options = {
      enable = mkEnableOption "this sandbox server" // { default = true; };

      address = mkOption {
        type = types.str;
        default = cfg.address;
        defaultText = lib.literalExpression "config.services.mftr.address";
        description = "Address to listen on.";
      };

      port = mkOption {
        type = types.port;
        description = "UDP port (each server needs its own).";
      };

      scenario = mkOption {
        type = types.enum [ "duel" "minions" "dodge" "empty" "aram" "mayhem" "hyper" "classic" "horde" ];
        default = "duel";
        description = ''
          The one scenario this server runs (`mftr-server --scenario`). `duel`: the Duel
          Sandbox. `dodge`: the blind playtest's dodge rig. `minions`, `empty`: test grounds.
          A full-match scenario here is a single game with these settings, apart from the
          main server's queues.
        '';
      };

      maxPlayers = mkOption {
        type = types.ints.between 1 255;
        default = 10;
        description = "Players (humans and bots) per game. Up to 8 spectators come on top.";
      };

      bots = mkOption {
        type = types.ints.u8;
        default = 0;
        description = "Server bots (they count toward `maxPlayers`).";
      };

      lobby = mkOption {
        type = types.bool;
        default = false;
        description = "Champion select before each match (full-match scenarios).";
      };

      seed = mkOption {
        type = types.ints.unsigned;
        default = 1;
        description = "Seeds everything random in the game.";
      };

      replay = mkOption {
        type = types.bool;
        default = false;
        description = "Record each match into /var/lib/mftr/${name}/replays.";
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
in
{
  imports = [
    (lib.mkRemovedOptionModule [ "services" "mftr" "servers" ] ''
      One MFTR server now plays every full-match mode on one port (players pick what to play;
      see docs/hosting.md). Use services.mftr.{port,modes,maps,replay}, and
      services.mftr.sandboxes.<name> for the Duel Sandbox and the other test grounds (a sandbox
      named like an old server keeps its key). To keep the main port's key that players pinned,
      set services.mftr.adoptKeyFrom to the old server's name, e.g. "aram".
    '')
  ];

  options.services.mftr = {
    enable = mkEnableOption "MFTR game servers";

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
      description = "The main server's UDP port.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open the main server's UDP port in the firewall (and sandboxes', by default).";
    };

    modes = mkOption {
      type = types.nonEmptyListOf (types.enum (fullModes ++ lib.concatMap (m: [ "${m}-versus" "${m}-coop" ]) fullModes ++ [ "horde" "horde-coop" ]));
      default = fullModes ++ [ "horde" ];
      example = [ "aram" "mayhem-coop" "classic-versus" ];
      description = ''
        What players can pick. A mode alone offers it Versus (humans on both teams, bots in
        the empty seats) and Co-op vs bots (humans on blue); `<mode>-versus` or `<mode>-coop`
        offers one side only. `aram`: The Bridge, a full match with a winner. `mayhem`: ARAM
        with augment drafts. `hyper`: Mayhem under Hyper rules. `classic`: Classic 5v5 on
        Crossroads (and on each of `maps`). Those matches have 10 seats and champion select.
        `horde` or `horde-coop`: Mayhem: Horde, five blue champions against 20 minion waves,
        then endless survival. Horde is co-op only, with champion picks and bot-filled seats.
      '';
    };

    maps = mkOption {
      type = types.listOf types.path;
      default = [ ];
      example = lib.literalExpression "[ ./maps/twin-rivers.mftrmap ]";
      description = ''
        Map files exported from the map editor: each adds its modes on that map (Classic on a
        Classic map; ARAM, Mayhem, Hyper and Horde on an ARAM map), on the sides `modes` gives them. Players download a map as they join. The server refuses to
        start if a map breaks a rule.
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

    botTier = mkOption {
      type = types.enum [ "recruit" "veteran" "elite" "nightmare" ];
      default = "veteran";
      description = ''
        How well bots play (design doc 17 §3): where co-op champion select's vote starts, and
        the tier of bots that fill Versus seats or play for dropped players.
      '';
    };

    workers = mkOption {
      type = types.nullOr types.ints.positive;
      default = null;
      description = "Threads that step the matches (default: one per core).";
    };

    seed = mkOption {
      type = types.ints.unsigned;
      default = 1;
      description = "Seeds everything random in the matches (each match draws its own from it).";
    };

    keyFile = mkOption {
      type = types.str;
      default = "/var/lib/mftr/server/server.key";
      description = ''
        The main server's key, created on first start. Clients pin it. It must be inside
        /var/lib/mftr/server (the server can't read other servers' directories); to keep an
        older server's key, use `adoptKeyFrom`.
      '';
    };

    adoptKeyFrom = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "aram";
      description = ''
        The name of an older server (from the removed `services.mftr.servers`) whose key the
        main server takes on its first start, so players who pinned it still connect. Its key
        is copied from /var/lib/mftr/<name>; nothing happens once the main server has a key.
        Without it, a key from 0.11.0's layout (/var/lib/mftr/server.key) is taken if there.
      '';
    };

    replay = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Record every match into /var/lib/mftr/server/replays (rewritten every minute and when
        a match ends; check one with `mftr-tools replay`).
      '';
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "More `mftr-server` options for the main server.";
    };

    sandboxes = mkOption {
      type = types.attrsOf (types.submodule sandbox);
      default = { };
      example = lib.literalExpression ''
        {
          duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };
          dodge = { port = 7781; scenario = "dodge"; };
        }
      '';
      description = ''
        Servers that run one scenario each, apart from the main server: the Duel Sandbox, the
        dodge rig, test grounds. One systemd service `mftr-<name>` each, keeping its key in
        /var/lib/mftr/<name> (so a sandbox named like an older server keeps that server's key).
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions =
      let
        ports = [ cfg.port ] ++ lib.mapAttrsToList (_: s: s.port) enabledSandboxes;
      in
      [
        {
          assertion = lib.length ports == lib.length (lib.unique ports);
          message = "services.mftr: the main server and every enabled sandbox need their own port.";
        }
        {
          assertion = !(enabledSandboxes ? server);
          message = "services.mftr.sandboxes: `server` is the main server's name; pick another.";
        }
      ];

    # `mftr-tools replay`, `mftr-server --fingerprint` and the bot client on the command line.
    environment.systemPackages = [ cfg.package ];

    systemd.services = {
      mftr = service {
        name = "server";
        description = "MFTR game server";
        args = [
          "--bind" (bind cfg.address cfg.port)
          "--key" cfg.keyFile
          "--seed" (toString cfg.seed)
          "--modes" (lib.concatStringsSep "," cfg.modes)
          "--max-matches" (toString cfg.maxMatches)
          "--bot-tier" cfg.botTier
        ]
        ++ lib.optionals (cfg.workers != null) [ "--workers" (toString cfg.workers) ]
        ++ lib.concatMap (m: [ "--map" "${m}" ]) cfg.maps
        ++ lib.optionals cfg.replay [ "--replay" "/var/lib/mftr/server/replays" ]
        ++ cfg.extraArgs;
      };
      mftr-adopt-key = {
        description = "MFTR game server: take over an older server's key";
        requiredBy = [ "mftr.service" ];
        before = [ "mftr.service" ];
        serviceConfig = {
          Type = "oneshot";
          ExecStart = adoptKey;
        };
      };
    } // lib.mapAttrs' (name: s: lib.nameValuePair "mftr-${name}" (service {
      inherit name;
      description = "MFTR ${s.scenario} server (${name})";
      args = [
        "--bind" (bind s.address s.port)
        "--key" "/var/lib/mftr/${name}/server.key"
        "--scenario" s.scenario
        "--bots" (toString s.bots)
        "--max-players" (toString s.maxPlayers)
        "--seed" (toString s.seed)
      ]
      ++ lib.optional s.lobby "--lobby"
      ++ lib.optionals s.replay [ "--replay" "/var/lib/mftr/${name}/replays" ]
      ++ s.extraArgs;
    })) enabledSandboxes;

    networking.firewall.allowedUDPPorts =
      lib.optional cfg.openFirewall cfg.port
      ++ lib.mapAttrsToList (_: s: s.port) (lib.filterAttrs (_: s: s.openFirewall) enabledSandboxes);
  };
}
