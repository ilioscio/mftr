# NixOS VM test for the module: the main server takes over an older server's key on its first
# start, keeps it across a restart, and bots pick queues and play through the encrypted
# transport with the key pinned, each queue in its own match; a duel sandbox runs beside it with
# its own key. Run with `nix flake check` (or `nix build .#checks.x86_64-linux.nixos`).
{ module }:
{
  name = "mftr";

  nodes.machine = { lib, ... }: {
    imports = [ module ];
    services.mftr = {
      enable = true;
      openFirewall = true;
      modes = [ "aram" "classic-coop" ];
      replay = true;
      adoptKeyFrom = "mayhem";
      sandboxes.duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };
    };
    # The test starts the main server itself, once an old server's key is in place.
    systemd.services.mftr.wantedBy = lib.mkForce [ ];
  };

  testScript = ''
    machine.wait_for_unit("mftr-duel.service")
    machine.wait_until_succeeds("journalctl -u mftr-duel | grep -q 'key fingerprint'")

    # The layout before 0.11: a server named mayhem kept its key in /var/lib/mftr/mayhem.
    machine.succeed("mkdir -p /var/lib/private/mftr/mayhem")
    old = machine.succeed("mftr-server --key /var/lib/private/mftr/mayhem/server.key --fingerprint").strip()

    machine.systemctl("start mftr.service")
    machine.wait_for_unit("mftr.service")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'key fingerprint'")
    machine.succeed("journalctl -u mftr-adopt-key | grep -q 'adopted the server key'")
    machine.succeed(f"journalctl -u mftr | grep -q 'key fingerprint {old}'")
    machine.succeed("journalctl -u mftr | grep -q '3 queues'")

    # The key survives a restart: clients that pinned it still connect.
    key = "/var/lib/mftr/server/server.key"
    machine.systemctl("restart mftr.service")
    machine.wait_for_unit("mftr.service")
    machine.wait_until_succeeds("test $(journalctl -u mftr | grep -c 'listening on') -ge 2")
    assert machine.succeed(f"mftr-server --key {key} --fingerprint").strip() == old

    server = f"127.0.0.1:7777#{old}"
    machine.succeed(
        f"mftr-tools bot --server '{server}' --queue aram --seconds 5 & "
        f"mftr-tools bot --server '{server}' --queue classic-coop --seconds 5; wait"
    )
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'started: aram-versus-the-bridge'")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'started: classic-coop-crossroads'")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'stopped after'")
    machine.succeed("ls /var/lib/mftr/server/replays | grep -q 'aram-versus-the-bridge'")
    machine.fail("mftr-tools bot --server '127.0.0.1:7777#00000000000000000000000000000000' --seconds 2")

    # The sandbox: its own key, kept across a restart, and a sparring duel.
    duel = machine.succeed("mftr-server --key /var/lib/mftr/duel/server.key --fingerprint").strip()
    assert duel != old, "each server has its own key"
    machine.systemctl("restart mftr-duel.service")
    machine.wait_for_unit("mftr-duel.service")
    assert machine.succeed("mftr-server --key /var/lib/mftr/duel/server.key --fingerprint").strip() == duel
    machine.succeed(f"mftr-tools bot --server '127.0.0.1:7780#{duel}' --duel --seconds 3")

    machine.succeed("nft list ruleset | grep -q 7780 || iptables -S | grep -q 7780")
    machine.succeed("nft list ruleset | grep -q 7777 || iptables -S | grep -q 7777")
  '';
}
