#!/usr/bin/env bash
# Print an ffmpeg concat list that times each VHS frame by its capture time.
#
#   ./scripts/vhs-concat.sh <frames-dir> <text|cursor> <fps>
#
# VHS drops frames when it cannot capture fast enough, so a fixed input rate
# plays a busy recording too fast. The file times of the text frames keep it in
# real time. A gap over half a second comes from a hidden section or a capture
# stall, so it shrinks to one frame at <fps>.

set -euo pipefail

frames=$1
layer=$2
fps=$3

perl -MTime::HiRes=stat -e '
  my ($layer, $fps, @texts) = @ARGV;
  my @times = map { (stat $_)[9] } @texts;
  for my $index (0 .. $#texts) {
    my $gap = $index < $#texts ? $times[$index + 1] - $times[$index] : 1 / $fps;
    (my $frame = $texts[$index]) =~ s/frame-text-/frame-$layer-/;
    printf "file %s\nduration %.6f\n", $frame, $gap > 0.5 ? 1 / $fps : $gap;
  }
' "$layer" "$fps" "$frames"/frame-text-*.png
