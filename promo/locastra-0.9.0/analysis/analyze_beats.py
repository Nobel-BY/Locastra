import json
import pathlib
import sys

import librosa
import numpy as np
from scipy.signal import butter, sosfilt


audio_path = pathlib.Path(sys.argv[1])
output_path = pathlib.Path(sys.argv[2])
y, sr = librosa.load(audio_path, sr=None, mono=True)
percussive = librosa.effects.hpss(y)[1]
tempo, beat_times = librosa.beat.beat_track(
    y=percussive, sr=sr, tightness=400, units="time"
)

i = np.arange(len(beat_times))
A = np.vstack([i, np.ones_like(i)]).T
(interval, beat_zero), *_ = np.linalg.lstsq(A, beat_times, rcond=None)
grid = beat_zero + i * interval
residual = beat_times - grid


def band_env(lo: float, hi: float):
    sos = butter(4, [lo, hi], btype="band", fs=sr, output="sos")
    env = librosa.onset.onset_strength(y=sosfilt(sos, percussive), sr=sr)
    return env, librosa.times_like(env, sr=sr)


kick_env, env_times = band_env(40, 160)
strengths = []
for n, t in enumerate(grid):
    idx = int(np.argmin(np.abs(env_times - t)))
    strengths.append({"beat": int(n), "time": float(t), "strength": float(kick_env[idx])})

rms = librosa.feature.rms(y=y)[0]
rms_times = librosa.times_like(rms, sr=sr)
top_hits = sorted(strengths, key=lambda x: x["strength"], reverse=True)[:24]

payload = {
    "source": audio_path.name,
    "sample_rate": sr,
    "duration": float(len(y) / sr),
    "detected_tempo": float(np.asarray(tempo).reshape(-1)[0]),
    "bpm": float(60.0 / interval),
    "beat_zero": float(beat_zero),
    "beat_interval": float(interval),
    "max_residual_ms": float(np.max(np.abs(residual)) * 1000),
    "mean_abs_residual_ms": float(np.mean(np.abs(residual)) * 1000),
    "beats": [float(x) for x in grid],
    "top_kick_hits": top_hits,
    "rms": [
        {"time": float(rms_times[j]), "value": float(rms[j])}
        for j in range(0, len(rms), max(1, len(rms) // 500))
    ],
}
output_path.parent.mkdir(parents=True, exist_ok=True)
output_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({k: payload[k] for k in ["bpm", "beat_zero", "beat_interval", "max_residual_ms", "mean_abs_residual_ms"]}, indent=2))
print("top hits:", top_hits[:10])
