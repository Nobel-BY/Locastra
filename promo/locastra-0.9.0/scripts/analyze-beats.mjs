import fs from "node:fs";
import path from "node:path";

const [input, outputDir] = process.argv.slice(2);
if (!input || !outputDir) {
  throw new Error("Usage: node analyze-beats.mjs <mono-s16le.raw> <output-dir>");
}

const sampleRate = 22050;
const hop = 256;
const pcm = fs.readFileSync(input);
const samples = new Int16Array(pcm.buffer, pcm.byteOffset, Math.floor(pcm.byteLength / 2));
const energy = [];
for (let start = 0; start + hop <= samples.length; start += hop) {
  let sum = 0;
  for (let i = start; i < start + hop; i += 1) {
    const value = samples[i] / 32768;
    sum += value * value;
  }
  energy.push(Math.sqrt(sum / hop));
}

const novelty = energy.map((value, index) => Math.max(0, value - (energy[index - 1] ?? value)));
const mean = novelty.reduce((a, b) => a + b, 0) / novelty.length;
const centered = novelty.map((value) => Math.max(0, value - mean * 0.7));
const fps = sampleRate / hop;

let best = { bpm: 123, score: -Infinity, lag: 0 };
for (let bpm = 118; bpm <= 128; bpm += 0.05) {
  const lag = Math.round((60 / bpm) * fps);
  let score = 0;
  for (let i = lag; i < centered.length; i += 1) score += centered[i] * centered[i - lag];
  if (score > best.score) best = { bpm, score, lag };
}

let bestPhase = { phase: 0, score: -Infinity };
for (let phase = 0; phase < best.lag; phase += 1) {
  let score = 0;
  for (let i = phase; i < centered.length; i += best.lag) {
    score += centered[i];
  }
  if (score > bestPhase.score) bestPhase = { phase, score };
}

const duration = samples.length / sampleRate;
const beatPeriod = 60 / best.bpm;
const firstBeat = bestPhase.phase / fps;
const beats = [];
for (let t = firstBeat; t <= duration; t += beatPeriod) beats.push(Number(t.toFixed(4)));
const barDownbeats = beats.filter((_, index) => index % 4 === 0);
const sourceStartFromFrames = 7;
const sourceOffsetSeconds = sourceStartFromFrames / 30;
const outputBeats = beats.map((beat) => Number((beat - sourceOffsetSeconds).toFixed(4))).filter((beat) => beat >= 0);
const targetCuts = [0, 104 / 30, 237 / 30, 385 / 30, 503 / 30, 740 / 30, 30];
const snappedCuts = targetCuts.map((target) => {
  if (target === 0) return {target, beat: 0, driftMs: 0};
  const nearest = outputBeats.reduce((a, b) => (Math.abs(b - target) < Math.abs(a - target) ? b : a), outputBeats[0]);
  return { target, beat: nearest, driftMs: Math.round((nearest - target) * 1000) };
});

fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(
  path.join(outputDir, "beat_data.json"),
  JSON.stringify({ sampleRate, duration, bpm: Number(best.bpm.toFixed(2)), beatPeriod, firstBeat, sourceStartFromFrames, sourceOffsetSeconds, sourceBeats: beats, outputBeats, barDownbeats }, null, 2),
);
fs.writeFileSync(
  path.join(outputDir, "grid_drift.json"),
  JSON.stringify({ targetCuts, snappedCuts, maxAbsDriftMs: Math.max(...snappedCuts.map((item) => Math.abs(item.driftMs))) }, null, 2),
);
console.log(JSON.stringify({ bpm: Number(best.bpm.toFixed(2)), firstBeat, snappedCuts }, null, 2));
