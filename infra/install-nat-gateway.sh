#!/bin/sh
# Install a built service and the existing private protocol artifact on a node.
set -eu
if [ "$#" -ne 2 ]; then
    echo "usage: $0 <gateway-binary> <artifacts.json>" >&2
    exit 2
fi
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v nft >/dev/null
command -v systemd-sysusers >/dev/null
install -m 0644 "$script_dir/rokbattles-nat-gateway.sysusers" /usr/lib/sysusers.d/rokbattles-nat-gateway.conf
systemd-sysusers /usr/lib/sysusers.d/rokbattles-nat-gateway.conf
install -d -m 0700 -o rokb-gateway -g rokb-gateway /var/lib/rokbattles-nat-gateway
install -d -m 0755 /opt/rokbattles-nat-gateway/artifacts
install -m 0644 "$2" /opt/rokbattles-nat-gateway/artifacts/artifacts.json
# Renaming permits installation while the old executable is still running.
install -m 0755 "$1" /usr/local/bin/rokbattles-nat-gateway.new
mv /usr/local/bin/rokbattles-nat-gateway.new /usr/local/bin/rokbattles-nat-gateway
install -m 0644 "$script_dir/rokbattles-nat-gateway.service" /etc/systemd/system/rokbattles-nat-gateway.service
install -d -m 0755 /etc/rokbattles
if [ ! -e /etc/rokbattles/nat-gateway.env ]; then
    install -m 0600 "$script_dir/../crates/apps/rokbattles-nat-gateway/.env.example" /etc/rokbattles/nat-gateway.env
fi
systemctl daemon-reload
echo 'Set UPSTREAM_ADDR and RELAY_TOKEN in /etc/rokbattles/nat-gateway.env, then enable or restart rokbattles-nat-gateway.service.'
