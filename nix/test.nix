# NixOS VM test for the module: two servers come up, keep their keys across a restart, and a
# bot plays through the encrypted transport with the key pinned. Run with `nix flake check`
# (or `nix build .#checks.x86_64-linux.nixos`).
{ module }:
{
  name = "mftr";

  nodes.machine = {
    imports = [ module ];
    services.mftr = {
      enable = true;
      openFirewall = true;
      servers = {
        aram = { port = 7777; bots = 9; replay = true; };
        duel = { port = 7778; scenario = "duel"; maxPlayers = 4; };
      };
    };
  };

  testScript = ''
    machine.wait_for_unit("mftr-aram.service")
    machine.wait_for_unit("mftr-duel.service")
    machine.wait_until_succeeds("journalctl -u mftr-aram | grep -q 'key fingerprint'")

    key = "/var/lib/mftr/aram/server.key"
    fp = machine.succeed(f"mftr-server --key {key} --fingerprint").strip()
    machine.succeed(f"journalctl -u mftr-aram | grep -q 'key fingerprint {fp}'")

    # The key survives a restart: clients that pinned it still connect.
    machine.systemctl("restart mftr-aram.service")
    machine.wait_for_unit("mftr-aram.service")
    machine.wait_until_succeeds("test $(journalctl -u mftr-aram | grep -c 'listening on') -ge 2")
    assert machine.succeed(f"mftr-server --key {key} --fingerprint").strip() == fp

    machine.succeed(f"mftr-tools bot --server '127.0.0.1:7777#{fp}' --seconds 5")
    machine.wait_until_succeeds("journalctl -u mftr-aram | grep -q 'player .* connected'")
    machine.fail("mftr-tools bot --server '127.0.0.1:7777#00000000000000000000000000000000' --seconds 2")

    duel = machine.succeed("mftr-server --key /var/lib/mftr/duel/server.key --fingerprint").strip()
    assert duel != fp, "each server has its own key"
    machine.succeed(f"mftr-tools bot --server '127.0.0.1:7778#{duel}' --duel --seconds 3")

    machine.succeed("nft list ruleset | grep -q 7778 || iptables -S | grep -q 7778")
  '';
}
