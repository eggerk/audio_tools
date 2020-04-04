#!/bin/sh

usage() {
  echo "Usage: audio_tools.sh <command>

  Commmands:
    raise           Raise volume by 5%.
    lower           Lower volume by 5%.
    mute            Mute/unmute audio.
    cycle-inputs    Change to next audio device.
    show            Show current volume.
    help            Display this help text."
}

arg="$1"
if [ -z "$arg" ]; then
  echo "No command given." >&2
  echo
  usage
  exit 1
fi
case "$arg" in
raise)
  target=VolumeRaise
  ;;
lower)
  target=VolumeLower
  ;;
mute)
  target=VolumeToggleMute
  ;;
cycle-inputs)
  target=CycleInputs
  ;;
show)
  target=ShowVolume
  ;;
h|help|-h|-help|--help)
  usage
  exit 0
  ;;
*)
  echo "Unkown command '$1'." >&2
  echo
  usage
  exit 2
  ;;
esac

dbus-send --print-reply --reply-timeout=120000 --type=method_call \
    --dest=ch.eggerk.volume_notification /volume_control \
    ch.eggerk.volume_notification."$target"
