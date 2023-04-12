#!/usr/bin/sh

convert white/single.png -channel RGB -negate black/single.png
convert white/triple.png -channel RGB -negate black/triple.png
convert white/polar.png -channel RGB -negate black/polar.png
convert white/L.png -channel RGB -negate black/L.png
convert white/empty.png -channel RGB -negate black/empty.png
