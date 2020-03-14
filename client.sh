#!/bin/sh

arg="$1"
if [ -z "$arg" ]; then
  arg="-h"
fi
case "$arg" in
raise)
  # Raise
  target=VolumeRaise
  ;;
lower)
  # Lower
  target=VolumeLower
  ;;
mute)
  # Mute
  target=VolumeToggleMute
  ;;
cycle-inputs)
  # Change output
  target=CycleInputs
  ;;
help|-h|--help)
  echo "Usage: audio_tools.sh <command>

  Commmands:
    raise           Raise volume by 5%.
    lower           Lower volume by 5%.
    mute            Mute/unmute audio.
    cycle-inputs    Change to next audio device.
    help            Display this help text."
  ;;
esac

dbus-send --print-reply --reply-timeout=120000 --type=method_call \
    --dest=ch.eggerk.volume_notification /volume_control \
    ch.eggerk.volume_notification."$target"
