#!/bin/sh
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

desktop_file=data/applications/io.github.kakadon000.Highwater.desktop
autostart_file=data/autostart/io.github.kakadon000.Highwater.desktop
metainfo_file=data/metainfo/io.github.kakadon000.Highwater.metainfo.xml
service_file=data/dbus-1/services/io.github.kakadon000.Highwater.Daemon.service

desktop-file-validate "$desktop_file" "$autostart_file"
appstreamcli validate --pedantic --no-net "$metainfo_file"

grep -Fxq 'Name=io.github.kakadon000.Highwater.Daemon' "$service_file"
grep -Fxq 'Exec=/usr/bin/highwaterd' "$service_file"
grep -Fxq 'SystemdService=highwaterd.service' "$service_file"

for size in 16 22 24 32 48 64 128 256 512; do
    icon="data/icons/hicolor/${size}x${size}/apps/io.github.kakadon000.Highwater.png"
    file "$icon" | grep -Fq "PNG image data, ${size} x ${size},"
done

hidpi_icon=data/icons/hicolor/512x512@2/apps/io.github.kakadon000.Highwater.png
file "$hidpi_icon" | grep -Fq 'PNG image data, 1024 x 1024,'

# Symbolic marks are filled outlines: the icon-theme convention, which renderers that
# recolour symbolic icons by fill alone depend on, and which keeps every
# mark the same weight when the client colours it. Outline the strokes instead.
stroked=$(grep -lE 'stroke[=:]' data/icons/hicolor/symbolic/apps/*.svg || true)
if [ -n "$stroked" ]; then
    printf 'symbolic marks must be drawn as filled outlines, not strokes:\n%s\n' "$stroked" >&2
    exit 1
fi
