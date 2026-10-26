#!/bin/sh
# Builds a throw-away btrfs on a loopback file holding what btrbk leaves on
# a NAS: received snapshots of two subvolumes over several days, one of
# them incomplete, quotas on. Needs root.
#
#   sudo tools/make-fixture.sh DIR      creates DIR/fixture.img, mounts DIR/mnt
#   sudo tools/make-fixture.sh -u DIR   unmounts and deletes it
set -eu

if [ "${1:-}" = "-u" ]; then
    dir=${2:?usage: make-fixture.sh -u DIR}
    umount "$dir/mnt" 2>/dev/null || true
    rm -rf "${dir:?}/mnt" "${dir:?}/fixture.img"
    exit 0
fi

dir=$1
img="$dir/fixture.img"
mnt="$dir/mnt"
mkdir -p "$mnt"
truncate -s 1G "$img"
mkfs.btrfs -q -f "$img"
mount -o loop "$img" "$mnt"
btrfs quota enable "$mnt"

# the sending side: a PC with @ and @home
mkdir -p "$mnt/pc/snaps" "$mnt/volume1/backup/PC-Test-btrfs"
btrfs -q subvolume create "$mnt/pc/@"
btrfs -q subvolume create "$mnt/pc/@home"
mkdir -p "$mnt/pc/@/etc" "$mnt/pc/@home/alice/Documents/projects"
echo "UUID=fake / btrfs defaults 0 0" > "$mnt/pc/@/etc/fstab"
ln -s ../usr/share/zoneinfo/Europe/Rome "$mnt/pc/@/etc/localtime"

target="$mnt/volume1/backup/PC-Test-btrfs"
prev=""
for day in 20 21 22 23 24 25 26; do
    stamp="202610${day}T0900"
    dd if=/dev/urandom of="$mnt/pc/@home/alice/Documents/report-$day.bin" bs=64k count=$((day - 18)) status=none
    echo "note of day $day" > "$mnt/pc/@home/alice/Documents/note $day é.txt"
    echo "$day" > "$mnt/pc/@home/alice/Documents/projects/$(printf 'a-very-long-name-%.0s' 1 2 3 4 5 6)$day.txt"
    for sub in @ @home; do
        btrfs -q subvolume snapshot -r "$mnt/pc/$sub" "$mnt/pc/snaps/$sub.$stamp"
        if [ -n "$prev" ]; then
            btrfs -q send -p "$mnt/pc/snaps/$sub.$prev" "$mnt/pc/snaps/$sub.$stamp" | btrfs -q receive "$target"
        else
            btrfs -q send "$mnt/pc/snaps/$sub.$stamp" | btrfs -q receive "$target"
        fi
    done
    prev=$stamp
done

# btrbk's retention pruned the oldest @home snapshot on the target
btrfs -q subvolume delete "$target/@home.20261020T0900"
# a receive interrupted half way: writable, no received UUID
btrfs -q subvolume create "$target/@home.20261026T2100"
# a plain directory that only looks like a snapshot, and unrelated clutter
mkdir "$target/@.20261019T0900" "$target/test-acl"
# the PC pruned its older snapshots: the newest one left on both sides is
# btrbk's next parent; then the PC reports what it holds, as
# extras/pc/btrbk-nas-push-uuids does
for day in 20 21 22 23; do
    btrfs -q subvolume delete "$mnt/pc/snaps/@.202610${day}T0900" "$mnt/pc/snaps/@home.202610${day}T0900"
done
btrfs -q subvolume snapshot -r "$mnt/pc/@home" "$mnt/pc/snaps/@home.20261026T2200"
{
    printf '# btrbk-nas source v1\nhost PC-Test\ngenerated %s\n' "$(date +%s)"
    for snap in "$mnt"/pc/snaps/*; do
        uuid=$(btrfs subvolume show "$snap" | awk '$1 == "UUID:" { print $2 }')
        printf 'snapshot %s %s\n' "$uuid" "$(basename "$snap")"
    done
} > "$target/.btrbk-nas-source"

btrfs quota rescan -w "$mnt" >/dev/null
echo "$target"
