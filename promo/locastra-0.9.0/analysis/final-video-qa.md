# Final video QA

## Deliverables

| Edition | Frame | Duration | Video | Audio |
| --- | --- | --- | --- | --- |
| Bilibili | 1920x1080 @ 30 fps | 30.059 s | H.264, 9.93 Mbps | AAC |
| Douyin | 1080x1920 @ 30 fps | 30.059 s | H.264, 9.93 Mbps | AAC |

Both BGM and no-BGM masters use `yuv420p`, limited TV range, and bitstream-level BT.709 matrix, transfer, and primaries metadata.

## Timing and visual checks

- Music analysis: **121.6 BPM**.
- Narrative cut drift: **14 ms maximum** for all non-terminal cuts.
- Scene overlaps eliminate black transition frames.
- Download progress is consistent and capped at 38%.
- CUDA / Vulkan is shown explicitly.
- Chat content is a fictional safe demo and contains no local paths or prior user text.
- Vertical edition uses contained desktop captures rather than center-cropping.
- Outro includes `0.9.0 · 现在开源 · WINDOWS` and has no residual capability pills.
