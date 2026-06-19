# Home Assistant Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Home Assistant 2026.4.2 and its Matter fabric from the Orange Pi at `192.168.2.62` to a low-resource HAOS VM on the M1 Mac, with bridged mDNS/HomeKit and a preserved rollback path.

**Architecture:** UTM runs ARM64 HAOS through Apple Virtualization.framework with 1 vCPU, 1 GB RAM, and a 16 GB sparse disk. Restore happens on UTM Shared Network so the new instance cannot advertise on the physical LAN; cutover powers off the Orange Pi, bridges the VM to `en5`, and assigns HAOS `192.168.2.62/24`. A native HA backup restores Core state; a separately verified archive restores the external Matter Server data.

**Tech Stack:** Home Assistant OS ARM64, UTM, Apple Virtualization.framework, `utmctl`, HAOS Supervisor CLI, Docker, launchd, zsh, `taskpolicy`, SSH/SCP, `dns-sd`.

## Global Constraints

- VM: exactly 1 vCPU, initially 1024 MiB RAM, and 16 GiB dynamically allocated disk.
- Increase RAM to 1536 MiB only after observed OOM, restart, Matter instability, or sustained memory pressure; use 2048 MiB only if 1536 MiB fails.
- Use UTM Apple Virtualization, never emulation/QEMU; bridge only to wired `en5`.
- Final network: `192.168.2.62/24`, gateway and DNS `192.168.2.1`.
- Orange Pi and HAOS must never be active on `192.168.2.62` simultaneously.
- Never delete source data. Stop and power off the source only at the explicit cutover checkpoint.
- Secrets and backups stay under `/Users/server/Backups/HomeAssistantMigration` with mode `0700`, outside Git and tool output.
- Preserve all unrelated changes already in `/Users/server/Dev/UniFi`.

## File Map

- Create: `ops/home-assistant/start-home-assistant-vm.sh` — idempotent background-QoS VM starter.
- Create: `ops/home-assistant/tests/start-home-assistant-vm.test.sh` — fake-`utmctl` behavior test.
- Create: `ops/home-assistant/com.local.home-assistant-vm.plist` — source-controlled LaunchDaemon.
- Install: `/Users/server/Library/Application Support/HomeAssistantVM/start-home-assistant-vm.sh`.
- Install: `/Library/LaunchDaemons/com.local.home-assistant-vm.plist`.
- Create confidential state: `/Users/server/Backups/HomeAssistantMigration/YYYYMMDD-HHMMSS/`.
- Create external VM state: UTM VM named `Home Assistant`.

---

### Task 1: Preflight, UTM, and HAOS Image

**Files:**
- Create: `/Users/server/Backups/HomeAssistantMigration/YYYYMMDD-HHMMSS/haos.img`

**Interfaces:**
- Consumes: M1 Mac, `en5`, GitHub HAOS releases API.
- Produces: installed UTM, verified ARM64 HAOS image, `/private/tmp/ha-migration-dir`.

- [ ] **Step 1: Verify host and create the protected migration directory**

```bash
test "$(uname -m)" = arm64
test "$(ifconfig en5 | awk '/status:/{print $2}')" = active
test "$(netstat -rn -f inet | awk '$1=="default" && $4=="en5" {print $2; exit}')" = 192.168.2.1
MIGRATION_DIR="/Users/server/Backups/HomeAssistantMigration/$(date +%Y%m%d-%H%M%S)"
install -d -m 0700 "$MIGRATION_DIR"
printf '%s\n' "$MIGRATION_DIR" > /private/tmp/ha-migration-dir
```

Expected: all tests exit 0 and `stat -f %Sp "$MIGRATION_DIR"` prints `drwx------`.

- [ ] **Step 2: Install UTM and verify required CLI commands**

Run with installation approval:

```bash
brew install --cask utm
/Applications/UTM.app/Contents/MacOS/utmctl help
```

Expected: help lists `list`, `start`, `stop`, `status`, and `ip-address`. Stop if any is absent because autostart depends on them.

- [ ] **Step 3: Download and verify current generic ARM64 HAOS**

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
ASSET_URL="$(curl -fsSL https://api.github.com/repos/home-assistant/operating-system/releases/latest | jq -r '.assets[] | select(.name|test("^haos_generic-aarch64-[0-9.]+\\.img\\.xz$")) | .browser_download_url')"
test -n "$ASSET_URL"
curl -fL "$ASSET_URL" -o "$MIGRATION_DIR/haos.img.xz"
shasum -a 256 "$MIGRATION_DIR/haos.img.xz" | tee "$MIGRATION_DIR/haos.img.xz.sha256"
xz -dk "$MIGRATION_DIR/haos.img.xz"
test "$(stat -f %z "$MIGRATION_DIR/haos.img")" -gt 1000000000
```

Expected: the compressed image has a recorded digest and the raw image is larger than 1 GB.

### Task 2: Create the Isolated HAOS VM

**Files:**
- Create: UTM VM bundle named `Home Assistant`.

**Interfaces:**
- Consumes: `$MIGRATION_DIR/haos.img`.
- Produces: booted HAOS on Shared Network and `/private/tmp/ha-nat-ip`.

- [ ] **Step 1: Prove no VM will be overwritten**

```bash
! /Applications/UTM.app/Contents/MacOS/utmctl list | grep -F 'Home Assistant'
```

Expected: exit 0. If the VM exists, inspect it and stop rather than delete it.

- [ ] **Step 2: Create the VM in UTM**

Choose **Create → Virtualize → Other**, then configure exactly:

```text
Name: Home Assistant
Backend: Apple Virtualization
Architecture: ARM64/aarch64
CPU: 1
Memory: 1024 MiB
Boot: UEFI
Primary disk: import $MIGRATION_DIR/haos.img, expand to 16 GiB
Network: Shared Network
Sharing/audio: disabled
```

Expected: UTM summary says Apple Virtualization, 1 CPU, 1024 MiB, 16 GiB, Shared Network. Do not proceed if it says Emulate or QEMU.

- [ ] **Step 3: Boot and prove Layer-2 isolation**

```bash
/Applications/UTM.app/Contents/MacOS/utmctl start "Home Assistant"
for i in {1..60}; do
  HA_NAT_IP="$(/Applications/UTM.app/Contents/MacOS/utmctl ip-address "Home Assistant" 2>/dev/null | awk 'NF{print $1; exit}')"
  [[ -n "$HA_NAT_IP" ]] && break
  sleep 5
done
test -n "$HA_NAT_IP"
[[ "$HA_NAT_IP" != 192.168.2.* ]]
printf '%s\n' "$HA_NAT_IP" > /private/tmp/ha-nat-ip
curl --retry 30 --retry-delay 5 --retry-connrefused -I "http://$HA_NAT_IP:8123"
```

Expected: HA onboarding responds from an address outside `192.168.2.0/24`.

### Task 3: Create and Validate Source Backups

**Files:**
- Create: `$MIGRATION_DIR/home-assistant-native.tar`.
- Create: `$MIGRATION_DIR/homeassistant-config-preflight.tar.gz`.
- Create: `$MIGRATION_DIR/matter-data-preflight.tar.gz`.

**Interfaces:**
- Consumes: source containers `homeassistant` and `matter-server`.
- Produces: one HA-native archive plus two independently readable raw archives.

- [ ] **Step 1: Create the native backup**

At `http://192.168.2.62:8123`, use **Settings → System → Backups → Create backup**, name it `pre-utm-migration`, include all HA data, and download it. Then run:

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
BACKUP="$(find /Users/server/Downloads -maxdepth 1 -type f -name '*.tar' -mmin -15 | sort | tail -1)"
test -n "$BACKUP"
mv "$BACKUP" "$MIGRATION_DIR/home-assistant-native.tar"
chmod 0600 "$MIGRATION_DIR/home-assistant-native.tar"
tar -tf "$MIGRATION_DIR/home-assistant-native.tar" | grep -E '(^|/)backup.json$'
```

Expected: archive contains `backup.json`; do not extract credential-bearing members.

- [ ] **Step 2: Take a short consistent raw rehearsal snapshot**

```bash
ssh orangepi@192.168.2.62 '
set -eu
mkdir -p /home/orangepi/ha-migration-preflight
docker stop homeassistant matter-server
docker run --rm --entrypoint tar -v /home/orangepi/homeassistant:/source:ro -v /home/orangepi/ha-migration-preflight:/backup ghcr.io/home-assistant/home-assistant:stable -C /source -czf /backup/homeassistant-config-preflight.tar.gz .
docker run --rm --entrypoint tar -v /home/orangepi/matter-server/data:/source:ro -v /home/orangepi/ha-migration-preflight:/backup ghcr.io/home-assistant/home-assistant:stable -C /source -czf /backup/matter-data-preflight.tar.gz .
docker start matter-server homeassistant
'
```

Expected: both archives are created and both source containers return to `Up`.

- [ ] **Step 3: Copy, validate, and hash raw archives**

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
scp orangepi@192.168.2.62:/home/orangepi/ha-migration-preflight/'*.tar.gz' "$MIGRATION_DIR/"
chmod 0600 "$MIGRATION_DIR/"*.tar.gz
tar -tzf "$MIGRATION_DIR/homeassistant-config-preflight.tar.gz" | grep -E '^\./configuration.yaml$'
tar -tzf "$MIGRATION_DIR/homeassistant-config-preflight.tar.gz" | grep -E '^\./\.storage/core.config_entries$'
test "$(tar -tzf "$MIGRATION_DIR/matter-data-preflight.tar.gz" | wc -l | tr -d ' ')" -gt 0
shasum -a 256 "$MIGRATION_DIR/"*.tar* > "$MIGRATION_DIR/backups.sha256"
```

Expected: both required HA files exist, Matter archive is nonempty, hashes are recorded.

### Task 4: Restore HA and Rehearse Matter Restore Under NAT

**Files:**
- Create: `/Users/server/Library/Application Support/HomeAssistantVM/haos-config.dmg`.
- Modify external state: restored HAOS Core and Matter Server app data.

**Interfaces:**
- Consumes: native backup, Matter raw archive, `/Users/server/.ssh/id_ed25519.pub`.
- Produces: restored LAN-isolated HAOS and root SSH access on port 22222.

- [ ] **Step 1: Restore native HA backup**

Open `http://$(cat /private/tmp/ha-nat-ip):8123`, select **Restore from backup**, upload `home-assistant-native.tar`, and wait for restart.

Expected: existing login works; Settings → About reports HAOS and Supervisor; restored entities are present. HomeKit/Matter can be unavailable while isolated.

- [ ] **Step 2: Install Matter Server app and stop it**

In **Settings → Apps**, install Matter Server, start once, then stop. At HAOS console run:

```text
ha addons info core_matter_server
```

Expected: slug `core_matter_server`, state `stopped`. Do not recreate the restored Matter integration.

- [ ] **Step 3: Create/import a CONFIG disk for root SSH**

```bash
install -d -m 0700 "/Users/server/Library/Application Support/HomeAssistantVM"
hdiutil create -size 16m -fs MS-DOS -volname CONFIG "/Users/server/Library/Application Support/HomeAssistantVM/haos-config.dmg"
hdiutil attach "/Users/server/Library/Application Support/HomeAssistantVM/haos-config.dmg"
cp /Users/server/.ssh/id_ed25519.pub /Volumes/CONFIG/authorized_keys
hdiutil detach /Volumes/CONFIG
```

Stop VM, attach the DMG as removable storage in UTM, boot, and run `ha os import` at HAOS console. Verify:

```bash
ssh -p 22222 -o BatchMode=yes root@"$(cat /private/tmp/ha-nat-ip)" 'ha info'
```

Expected: SSH exits 0. Fix CONFIG import before touching Supervisor data if it fails.

- [ ] **Step 4: Rehearse Matter restore and leave app stopped**

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
HA_NAT_IP="$(cat /private/tmp/ha-nat-ip)"
scp -P 22222 "$MIGRATION_DIR/matter-data-preflight.tar.gz" root@"$HA_NAT_IP":/tmp/
ssh -p 22222 root@"$HA_NAT_IP" '
set -eu
ha addons stop core_matter_server
DATA=/mnt/data/supervisor/addons/data/core_matter_server
test -d "$DATA"
find "$DATA" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
tar -xzf /tmp/matter-data-preflight.tar.gz -C "$DATA"
test "$(find "$DATA" -mindepth 1 | wc -l)" -gt 0
ha addons start core_matter_server
sleep 10
ha addons logs core_matter_server | tail -80
ha addons stop core_matter_server
'
```

Expected: Matter app starts without database/fabric corruption, then stops for cutover.

### Task 5: Build and Test Unattended Startup

**Files:**
- Create: `ops/home-assistant/start-home-assistant-vm.sh`.
- Create: `ops/home-assistant/tests/start-home-assistant-vm.test.sh`.
- Create: `ops/home-assistant/com.local.home-assistant-vm.plist`.

**Interfaces:**
- Produces: an idempotent starter and system LaunchDaemon.

- [ ] **Step 1: Write the failing test**

Create `ops/home-assistant/tests/start-home-assistant-vm.test.sh`:

```bash
#!/bin/zsh
set -euo pipefail
ROOT="${0:A:h:h}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cat > "$TMP/utmctl" <<'FAKE'
#!/bin/zsh
print -r -- "$*" >> "$FAKE_LOG"
[[ "$1" == status ]] && print -r -- "${FAKE_STATUS:-stopped}"
FAKE
chmod +x "$TMP/utmctl"
cat > "$TMP/taskpolicy" <<'FAKE'
#!/bin/zsh
[[ "$1" == -b ]] && shift
exec "$@"
FAKE
chmod +x "$TMP/taskpolicy"
export UTMCTL="$TMP/utmctl" TASKPOLICY="$TMP/taskpolicy" FAKE_LOG="$TMP/log"
FAKE_STATUS=started "$ROOT/start-home-assistant-vm.sh"
! grep -q '^start ' "$FAKE_LOG"
: > "$FAKE_LOG"
FAKE_STATUS=stopped "$ROOT/start-home-assistant-vm.sh"
grep -Fx 'start Home Assistant' "$FAKE_LOG"
UTMCTL="$TMP/missing" "$ROOT/start-home-assistant-vm.sh" >/dev/null 2>&1 && exit 1
print PASS
```

- [ ] **Step 2: Verify the test fails**

```bash
chmod +x ops/home-assistant/tests/start-home-assistant-vm.test.sh
ops/home-assistant/tests/start-home-assistant-vm.test.sh
```

Expected: FAIL because the startup wrapper does not exist.

- [ ] **Step 3: Implement the wrapper**

Create `ops/home-assistant/start-home-assistant-vm.sh`:

```bash
#!/bin/zsh
set -euo pipefail
UTMCTL="${UTMCTL:-/Applications/UTM.app/Contents/MacOS/utmctl}"
TASKPOLICY="${TASKPOLICY:-/usr/bin/taskpolicy}"
VM_NAME="Home Assistant"
[[ -x "$UTMCTL" ]] || { print -u2 "utmctl is not executable: $UTMCTL"; exit 69; }
"$UTMCTL" status "$VM_NAME" 2>/dev/null | grep -qi started && exit 0
exec "$TASKPOLICY" -b "$UTMCTL" start "$VM_NAME"
```

- [ ] **Step 4: Test the wrapper**

```bash
chmod +x ops/home-assistant/start-home-assistant-vm.sh
zsh -n ops/home-assistant/start-home-assistant-vm.sh ops/home-assistant/tests/start-home-assistant-vm.test.sh
ops/home-assistant/tests/start-home-assistant-vm.test.sh
```

Expected: prints `PASS`.

- [ ] **Step 5: Create and lint the LaunchDaemon**

Create `ops/home-assistant/com.local.home-assistant-vm.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.local.home-assistant-vm</string>
<key>ProgramArguments</key><array><string>/Users/server/Library/Application Support/HomeAssistantVM/start-home-assistant-vm.sh</string></array>
<key>UserName</key><string>server</string>
<key>GroupName</key><string>staff</string>
<key>RunAtLoad</key><true/>
<key>StandardOutPath</key><string>/Users/server/Library/Logs/HomeAssistantVM.out.log</string>
<key>StandardErrorPath</key><string>/Users/server/Library/Logs/HomeAssistantVM.err.log</string>
</dict></plist>
```

Run `plutil -lint ops/home-assistant/com.local.home-assistant-vm.plist`; expected: `OK`.

- [ ] **Step 6: Commit and install only these artifacts**

```bash
git add ops/home-assistant/start-home-assistant-vm.sh ops/home-assistant/tests/start-home-assistant-vm.test.sh ops/home-assistant/com.local.home-assistant-vm.plist
git commit -m "ops: add Home Assistant VM autostart"
install -m 0755 ops/home-assistant/start-home-assistant-vm.sh "/Users/server/Library/Application Support/HomeAssistantVM/start-home-assistant-vm.sh"
sudo install -o root -g wheel -m 0644 ops/home-assistant/com.local.home-assistant-vm.plist /Library/LaunchDaemons/com.local.home-assistant-vm.plist
sudo launchctl bootstrap system /Library/LaunchDaemons/com.local.home-assistant-vm.plist
sudo launchctl kickstart -k system/com.local.home-assistant-vm
sleep 5
sudo launchctl print system/com.local.home-assistant-vm | grep -E 'state =|last exit code ='
```

Expected: commit contains only three files; launchd last exit code is 0. If UTM requires an Aqua session, stop rather than enabling auto-login.

### Task 6: Final Backup and Cutover

**Files:**
- Create: `$MIGRATION_DIR/home-assistant-native-final.tar`.
- Create: `$MIGRATION_DIR/homeassistant-config-final.tar.gz`.
- Create: `$MIGRATION_DIR/matter-data-final.tar.gz`.

**Interfaces:**
- Produces: Orange Pi powered off; HAOS bridged to `en5` at `192.168.2.62`.

- [ ] **Step 1: Obtain explicit outage approval**

Report: `Source HA and Matter will stop, final archives will be copied, and orangepizero2w will power off. Rollback requires stopping the VM and powering the Orange Pi back on. Expected outage: 10–20 minutes.`

Expected: explicit approval. Design approval alone is not outage approval.

- [ ] **Step 2: Create the final native HA backup**

At the source `http://192.168.2.62:8123`, create and download a full backup named `final-utm-migration`, then run:

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
BACKUP="$(find /Users/server/Downloads -maxdepth 1 -type f -name '*.tar' -mmin -15 | sort | tail -1)"
test -n "$BACKUP"
mv "$BACKUP" "$MIGRATION_DIR/home-assistant-native-final.tar"
chmod 0600 "$MIGRATION_DIR/home-assistant-native-final.tar"
tar -tf "$MIGRATION_DIR/home-assistant-native-final.tar" | grep -E '(^|/)backup.json$'
```

Expected: final native archive is valid and protected.

- [ ] **Step 3: Create, copy, and validate final raw archives**

```bash
ssh orangepi@192.168.2.62 '
set -eu
mkdir -p /home/orangepi/ha-migration-final
docker stop homeassistant matter-server
docker run --rm --entrypoint tar -v /home/orangepi/homeassistant:/source:ro -v /home/orangepi/ha-migration-final:/backup ghcr.io/home-assistant/home-assistant:stable -C /source -czf /backup/homeassistant-config-final.tar.gz .
docker run --rm --entrypoint tar -v /home/orangepi/matter-server/data:/source:ro -v /home/orangepi/ha-migration-final:/backup ghcr.io/home-assistant/home-assistant:stable -C /source -czf /backup/matter-data-final.tar.gz .
'
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
scp orangepi@192.168.2.62:/home/orangepi/ha-migration-final/'*.tar.gz' "$MIGRATION_DIR/"
chmod 0600 "$MIGRATION_DIR/"*-final.tar.gz
tar -tzf "$MIGRATION_DIR/homeassistant-config-final.tar.gz" | grep '^\./\.storage/core.config_entries$'
test "$(tar -tzf "$MIGRATION_DIR/matter-data-final.tar.gz" | wc -l | tr -d ' ')" -gt 0
shasum -a 256 "$MIGRATION_DIR/"*-final.tar.gz >> "$MIGRATION_DIR/backups.sha256"
```

Expected: valid final archives, source containers remain stopped. On failure restart them and abort.

- [ ] **Step 4: Restore the final native HA backup into isolated HAOS**

At `http://$(cat /private/tmp/ha-nat-ip):8123`, open **Settings → System → Backups**, upload `home-assistant-native-final.tar`, restore all Home Assistant data, and wait for HA Core to restart.

Expected: existing login and integrations return on the NAT address; the source containers remain stopped. Confirm root SSH on port 22222 still works before continuing.

- [ ] **Step 5: Replace rehearsed Matter data with final data**

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
HA_NAT_IP="$(cat /private/tmp/ha-nat-ip)"
scp -P 22222 "$MIGRATION_DIR/matter-data-final.tar.gz" root@"$HA_NAT_IP":/tmp/
ssh -p 22222 root@"$HA_NAT_IP" '
set -eu
ha addons stop core_matter_server
DATA=/mnt/data/supervisor/addons/data/core_matter_server
find "$DATA" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
tar -xzf /tmp/matter-data-final.tar.gz -C "$DATA"
test "$(find "$DATA" -mindepth 1 | wc -l)" -gt 0
'
```

Expected: final Matter data is installed; app remains stopped.

- [ ] **Step 6: Set HAOS static network and shut it down**

In **Settings → System → Network**, configure the active Ethernet interface: IPv4 Static, `192.168.2.62/24`, gateway `192.168.2.1`, and DNS `192.168.2.1`. Applying this setting immediately makes the NAT address unreachable. Use the still-open UTM console and run:

```text
ha host shutdown
```

Expected: HAOS shuts down cleanly and `utmctl status "Home Assistant"` reports stopped. Do not force-stop the VM merely because the old NAT address no longer responds.

- [ ] **Step 7: Power off source and prove the IP is free**

```bash
ssh -t orangepi@192.168.2.62 'sudo /sbin/shutdown -h now'
for i in {1..30}; do ping -c 1 -W 500 192.168.2.62 >/dev/null 2>&1 || break; sleep 1; done
! ping -c 2 -W 500 192.168.2.62
```

Expected: source powers off and `.62` no longer responds. Never start the VM while it responds.

- [ ] **Step 8: Bridge only to en5 and start**

With VM stopped, edit UTM network to **Bridged → en5**, retaining its MAC. Run:

```bash
/Applications/UTM.app/Contents/MacOS/utmctl start "Home Assistant"
curl --retry 60 --retry-delay 5 --retry-connrefused -fsS -o /dev/null http://192.168.2.62:8123
```

Expected: HA responds at `.62` without IP conflict; start Matter Server app if Supervisor did not.

### Task 7: Acceptance, Cold Boot, and Rollback

**Files:**
- Create: `$MIGRATION_DIR/acceptance.txt` containing non-secret evidence.

**Interfaces:**
- Produces: accepted migration or restored Orange Pi service.

- [ ] **Step 1: Validate HA, integrations, Matter, and HomeKit**

Verify existing login, entities, history, Xiaomi Home, Neakasa, HACS, Matter, Thread, and HomeKit. Toggle one Matter light twice. Run `dns-sd -B _hap._tcp local.` for 15 seconds.

Expected: Matter works; HomeKit Bridge appears exactly once and resolves to `.62`. Do not create a new Matter fabric.

- [ ] **Step 2: Observe 30 minutes of stability**

```bash
MIGRATION_DIR="$(cat /private/tmp/ha-migration-dir)"
for i in {1..30}; do
  date
  curl -fsS -o /dev/null http://192.168.2.62:8123 && echo HA_HTTP=ok
  ps -axo pid,%cpu,rss,command | grep -E '[U]TM|[V]irtualization' | head -10
  sleep 60
done | tee "$MIGRATION_DIR/acceptance.txt"
memory_pressure | head -20 | tee -a "$MIGRATION_DIR/acceptance.txt"
```

Expected: 30 successful probes, no restarts/OOM, green host memory pressure. If OOM occurs, stop VM, set 1536 MiB, and repeat; use 2048 MiB only if 1536 MiB fails.

- [ ] **Step 3: Cold-boot test unattended startup**

Obtain explicit reboot approval, then run `sudo shutdown -r now`. After reboot, without opening UTM manually:

```bash
sudo launchctl print system/com.local.home-assistant-vm | grep -E 'state =|last exit code ='
/Applications/UTM.app/Contents/MacOS/utmctl status "Home Assistant"
curl --retry 60 --retry-delay 5 --retry-connrefused -fsS -o /dev/null http://192.168.2.62:8123
```

Expected: launchd exit 0, VM started, HA reachable without GUI login interaction.

- [ ] **Step 4: Use the exact rollback on any blocking failure**

```bash
/Applications/UTM.app/Contents/MacOS/utmctl stop "Home Assistant"
! ping -c 2 -W 500 192.168.2.62
```

Physically power on Orange Pi, then:

```bash
for i in {1..60}; do ssh -o ConnectTimeout=2 orangepi@192.168.2.62 true && break; sleep 5; done
ssh orangepi@192.168.2.62 'docker start matter-server homeassistant'
curl --retry 30 --retry-delay 5 --retry-connrefused -fsS -o /dev/null http://192.168.2.62:8123
```

Expected: old service returns. If acceptance passes, do not run rollback; keep Orange Pi powered off and preserve its data for separately approved cleanup.
