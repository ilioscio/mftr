# NixOS VM test for the module: the server comes up, keeps its key across a restart, and bots
# pick queues and play through the encrypted transport with the key pinned, each queue in its
# own match. Run with `nix flake check` (or `nix build .#checks.x86_64-linux.nixos`).
{ module }:
{
  name = "mftr";

  nodes.machine = {
    imports = [ module ];
    services.mftr = {
      enable = true;
      openFirewall = true;
      modes = [ "aram" "classic" ];
      replay = true;
    };
  };

  testScript = ''
    machine.wait_for_unit("mftr.service")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'key fingerprint'")
    machine.succeed("journalctl -u mftr | grep -q '4 queues'")

    key = "/var/lib/mftr/server.key"
    fp = machine.succeed(f"mftr-server --key {key} --fingerprint").strip()
    machine.succeed(f"journalctl -u mftr | grep -q 'key fingerprint {fp}'")

    # The key survives a restart: clients that pinned it still connect.
    machine.systemctl("restart mftr.service")
    machine.wait_for_unit("mftr.service")
    machine.wait_until_succeeds("test $(journalctl -u mftr | grep -c 'listening on') -ge 2")
    assert machine.succeed(f"mftr-server --key {key} --fingerprint").strip() == fp

    server = f"127.0.0.1:7777#{fp}"
    machine.succeed(
        f"mftr-tools bot --server '{server}' --queue aram --seconds 5 & "
        f"mftr-tools bot --server '{server}' --queue classic-coop --seconds 5; wait"
    )
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'started: aram-versus-the-bridge'")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'started: classic-coop-crossroads'")
    machine.wait_until_succeeds("journalctl -u mftr | grep -q 'stopped after'")
    machine.succeed("ls /var/lib/mftr/replays | grep -q 'aram-versus-the-bridge'")
    machine.fail("mftr-tools bot --server '127.0.0.1:7777#00000000000000000000000000000000' --seconds 2")

    machine.succeed("nft list ruleset | grep -q 7777 || iptables -S | grep -q 7777")
  '';
}
