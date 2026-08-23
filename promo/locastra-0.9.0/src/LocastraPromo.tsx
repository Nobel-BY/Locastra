import React from 'react';
import {
  AbsoluteFill,
  Audio,
  Img,
  Sequence,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';

const C = {
  bg: '#090f0f',
  canvas: '#0e1514',
  surface: '#161d1c',
  raised: '#1a2120',
  line: '#3c4948',
  cyan: '#62f9ed',
  cyanStrong: '#3cdcd1',
  text: '#dde4e2',
  muted: '#9eb0ad',
  warning: '#ffb956',
};

const ease = (v: number) => 1 - Math.pow(1 - v, 4);
const clamped = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;
const fade = (frame: number, duration: number) =>
  Math.min(interpolate(frame, [0, 12], [0, 1], clamped), interpolate(frame, [duration - 12, duration], [1, 0], clamped));

const Grid: React.FC = () => (
  <AbsoluteFill
    style={{
      backgroundColor: C.bg,
      backgroundImage:
        'linear-gradient(rgba(98,249,237,.035) 1px, transparent 1px), linear-gradient(90deg, rgba(98,249,237,.035) 1px, transparent 1px), radial-gradient(circle at 72% 28%, rgba(60,220,209,.10), transparent 34%)',
      backgroundSize: '64px 64px, 64px 64px, 100% 100%',
    }}
  />
);

const Eyebrow: React.FC<{children: React.ReactNode; vertical?: boolean}> = ({children, vertical}) => (
  <div style={{fontFamily: 'Cascadia Code, Consolas, monospace', fontSize: vertical ? 40 : 32, letterSpacing: vertical ? 6 : 5, color: C.cyan, fontWeight: 700}}>
    {children}
  </div>
);

const Title: React.FC<{children: React.ReactNode; vertical: boolean}> = ({children, vertical}) => (
  <div
    style={{
      fontFamily: 'Segoe UI Variable, Microsoft YaHei, sans-serif',
      fontSize: vertical ? 72 : 70,
      lineHeight: 1.08,
      fontWeight: 760,
      letterSpacing: -2,
      color: C.text,
    }}
  >
    {children}
  </div>
);

const Pill: React.FC<{children: React.ReactNode; accent?: boolean}> = ({children, accent}) => (
  <div
    style={{
      border: `1px solid ${accent ? C.cyanStrong : C.line}`,
      background: accent ? 'rgba(60,220,209,.12)' : 'rgba(22,29,28,.9)',
      color: accent ? C.cyan : C.muted,
      borderRadius: 999,
      padding: '12px 20px',
      font: '650 27px Segoe UI Variable, Microsoft YaHei, sans-serif',
      boxShadow: accent ? '0 0 30px rgba(60,220,209,.12)' : undefined,
      whiteSpace: 'nowrap',
    }}
  >
    {children}
  </div>
);

const Logo: React.FC<{size: number}> = ({size}) => (
  <div style={{width: size, height: size, overflow: 'hidden', borderRadius: size * 0.12, background: '#000', boxShadow: '0 0 70px rgba(98,249,237,.12)'}}>
    <Img src={staticFile('locastra-icon.png')} style={{width: '100%', height: '100%', filter: 'invert(1)', display: 'block'}} />
  </div>
);

const Opening: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const appear = spring({frame, fps: 30, config: {damping: 18, stiffness: 120}});
  const rule = ease(interpolate(frame, [8, 44], [0, 1], clamped));
  const letter = ease(interpolate(frame, [18, 56], [0, 1], clamped));
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), alignItems: 'center', justifyContent: 'center'}}>
      <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', transform: `translateY(${(1 - appear) * 35}px)`}}>
        <div style={{transform: `scale(${0.88 + appear * 0.12})`, opacity: appear}}><Logo size={vertical ? 200 : 165} /></div>
        <div style={{width: vertical ? 740 : 920, height: 1, background: C.line, marginTop: 48, position: 'relative'}}>
          <div style={{height: 2, width: `${rule * 100}%`, background: C.cyan, boxShadow: '0 0 20px rgba(98,249,237,.6)'}} />
        </div>
        <svg width={vertical ? 860 : 1040} height={vertical ? 150 : 165} viewBox="0 0 1040 165" style={{marginTop: 22, overflow: 'visible'}}>
          <text x="520" y="122" textAnchor="middle" style={{fontFamily: 'Segoe UI Variable, sans-serif', fontWeight: 800, fontSize: 112, letterSpacing: `${18 - letter * 7}px`, fill: `rgba(221,228,226,${interpolate(letter, [0.55, 1], [0, 1], clamped)})`, stroke: C.cyan, strokeWidth: 1.5, strokeDasharray: 1700, strokeDashoffset: 1700 * (1 - letter), filter: 'drop-shadow(0 0 12px rgba(98,249,237,.25))'}}>
            LOCASTRA
          </text>
        </svg>
        <Eyebrow vertical={vertical}>LOCAL AI · WINDOWS</Eyebrow>
      </div>
    </AbsoluteFill>
  );
};

const ScreenshotStage: React.FC<{src: string; vertical: boolean; zoom?: number; y?: number; scan?: number; children?: React.ReactNode}> = ({src, vertical, zoom = 1, y = 0, scan, children}) => {
  const boxWidth = vertical ? 970 : 1500;
  const boxHeight = vertical ? 610 : 840;
  return (
    <div style={{width: boxWidth, height: boxHeight, borderRadius: 24, overflow: 'hidden', position: 'relative', background: '#fff', border: `1px solid ${C.line}`, boxShadow: '0 38px 100px rgba(0,0,0,.48)'}}>
      <Img src={staticFile(src)} style={{width: '100%', height: '100%', objectFit: 'contain', objectPosition: 'center', background: C.canvas, transform: `scale(${zoom}) translateY(${y}px)`}} />
      <div style={{position: 'absolute', inset: 0, boxShadow: 'inset 0 0 0 1px rgba(255,255,255,.08)', pointerEvents: 'none'}} />
      {scan !== undefined ? <div style={{position: 'absolute', top: 0, bottom: 0, left: `${scan * 100}%`, width: 3, background: C.cyan, boxShadow: '0 0 28px 8px rgba(98,249,237,.35)'}} /> : null}
      {children}
    </div>
  );
};

const ProductScene: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const enter = spring({frame, fps: 30, config: {damping: 22, stiffness: 95}});
  const scan = interpolate(frame, [34, 72], [0, 1], clamped);
  const wireOpacity = interpolate(frame, [0, 48, 76], [0.9, 0.9, 0], clamped);
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), padding: vertical ? '140px 55px' : '78px 100px', alignItems: 'center'}}>
      <div style={{width: '100%', maxWidth: 1640}}>
        <Eyebrow vertical={vertical}>01 · MODEL DISCOVERY</Eyebrow>
        <div style={{marginTop: 16}}><Title vertical={vertical}>找到适合这台电脑的模型</Title></div>
      </div>
      <div style={{marginTop: vertical ? 120 : 48, transform: `translateY(${(1 - enter) * 80}px) scale(${0.96 + enter * 0.04})`, opacity: enter}}>
        <ScreenshotStage src="ui/model-search.png" vertical={vertical} scan={scan}>
          <div style={{position: 'absolute', inset: 0, opacity: wireOpacity, backgroundImage: 'linear-gradient(rgba(98,249,237,.12) 1px, transparent 1px), linear-gradient(90deg, rgba(98,249,237,.12) 1px, transparent 1px)', backgroundSize: '56px 56px', mixBlendMode: 'multiply'}} />
        </ScreenshotStage>
      </div>
    </AbsoluteFill>
  );
};

const ModelFitScene: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const swap = interpolate(frame, [62, 78], [0, 1], clamped);
  const pop = spring({frame: frame - 12, fps: 30, config: {damping: 18, stiffness: 130}});
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), padding: vertical ? '130px 55px' : '70px 100px', alignItems: 'center'}}>
      <div style={{width: '100%', maxWidth: 1640, display: 'flex', flexDirection: vertical ? 'column' : 'row', justifyContent: 'space-between', alignItems: vertical ? 'flex-start' : 'flex-end', gap: 30}}>
        <div><Eyebrow vertical={vertical}>02 · HARDWARE FIT</Eyebrow><div style={{marginTop: 14}}><Title vertical={vertical}>参数不用猜<br />Locastra 帮你选</Title></div></div>
        <div style={{display: 'flex', gap: 12, flexWrap: 'wrap', transform: `scale(${0.9 + pop * 0.1})`, opacity: pop}}><Pill accent>流畅运行</Pill><Pill>GGUF</Pill><Pill>Q4_K_M</Pill><Pill accent>CUDA / Vulkan 一键加载</Pill></div>
      </div>
      <div style={{marginTop: vertical ? 90 : 36, position: 'relative'}}>
        <div style={{opacity: 1 - swap}}><ScreenshotStage src="ui/model-search.png" vertical={vertical} zoom={1.03} /></div>
        <div style={{position: 'absolute', inset: 0, opacity: swap, clipPath: `inset(0 ${100 - swap * 100}% 0 0)`}}><ScreenshotStage src="ui/model-version.png" vertical={vertical} /></div>
        <div style={{position: 'absolute', left: vertical ? 44 : 300, right: vertical ? 44 : 260, top: vertical ? 145 : 225, height: vertical ? 64 : 72, borderRadius: 14, border: `1px solid ${C.cyanStrong}`, background: 'rgba(9,15,15,.96)', color: C.text, display: 'flex', alignItems: 'center', padding: '0 24px', font: `${vertical ? 30 : 28}px Cascadia Code, monospace`, boxShadow: '0 0 34px rgba(60,220,209,.16)', opacity: interpolate(frame, [6, 16, 70, 84], [0, 1, 1, 0], clamped)}}>
          <span style={{color: C.cyan, marginRight: 18}}>⌕</span>{'Qwen3 8B GGUF'.slice(0, Math.max(0, Math.floor(interpolate(frame, [12, 48], [0, 14], clamped))))}<span style={{opacity: frame % 16 < 8 ? 1 : 0, color: C.cyan}}>▌</span>
        </div>
      </div>
    </AbsoluteFill>
  );
};

const DownloadScene: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const p = 38;
  const progressReveal = interpolate(frame, [18, 54], [0, 1], clamped);
  const enter = spring({frame, fps: 30, config: {damping: 20, stiffness: 110}});
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), padding: vertical ? '140px 55px' : '74px 100px', alignItems: 'center'}}>
      <div style={{width: '100%', maxWidth: 1640}}><Eyebrow vertical={vertical}>03 · RESUMABLE DOWNLOAD</Eyebrow><div style={{marginTop: 14}}><Title vertical={vertical}>国内友好下载<br />中断后继续</Title></div></div>
      <div style={{marginTop: vertical ? 110 : 42, transform: `translateX(${(1 - enter) * 90}px)`, opacity: enter}}>
        <ScreenshotStage src="ui/download.png" vertical={vertical}>
          <div style={{position: 'absolute', left: '13%', right: '5%', top: vertical ? 170 : 255, height: vertical ? 180 : 220, borderRadius: 18, background: 'rgba(9,15,15,.97)', border: `1px solid ${C.line}`}} />
          {[0, 1, 2].map((i) => <div key={i} style={{position: 'absolute', left: `${16 + i * 5}%`, right: `${9 - i * 2}%`, top: `${vertical ? 23 + i * 4 : 34 + i * 4}%`, height: vertical ? 62 : 70, borderRadius: 14, background: i === 2 ? 'rgba(14,21,20,.98)' : 'rgba(22,29,28,.72)', border: `1px solid ${i === 2 ? C.cyanStrong : C.line}`, transform: `translateX(${Math.sin((frame + i * 20) / 18) * (i === 1 ? -10 : 10)}px)`, opacity: i === 2 ? 1 : .55}} />)}
          <div style={{position: 'absolute', left: '17%', right: '7%', bottom: vertical ? 115 : 145, borderRadius: 14, padding: '20px 22px', background: 'rgba(9,15,15,.98)', border: `1px solid ${C.line}`, color: C.text}}>
            <div style={{display: 'flex', justifyContent: 'space-between', font: '650 27px Segoe UI Variable, sans-serif'}}><span>断点续传 · 自动切换下载源 · 13 MB/s</span><span style={{color: C.cyan}}>{p.toFixed(0)}%</span></div>
            <div style={{height: 7, background: '#283231', borderRadius: 99, marginTop: 12, overflow: 'hidden'}}><div style={{width: `${p * progressReveal}%`, height: '100%', background: C.cyan, boxShadow: '0 0 18px rgba(98,249,237,.65)'}} /></div>
          </div>
        </ScreenshotStage>
      </div>
    </AbsoluteFill>
  );
};

const ChatScene: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const enter = spring({frame, fps: 30, config: {damping: 22, stiffness: 90}});
  const stream = interpolate(frame, [35, duration - 30], [0, 1], clamped);
  const words = ['完全本地', '流式响应', '会话留在设备'];
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), padding: vertical ? '135px 55px' : '72px 100px', alignItems: 'center'}}>
      <div style={{width: '100%', maxWidth: 1640, display: 'flex', flexDirection: vertical ? 'column' : 'row', justifyContent: 'space-between', alignItems: vertical ? 'flex-start' : 'flex-end', gap: 28}}>
        <div><Eyebrow vertical={vertical}>04 · LOCAL INFERENCE</Eyebrow><div style={{marginTop: 14}}><Title vertical={vertical}>加载完成<br />断网也能聊</Title></div></div>
        <div style={{display: 'flex', gap: 12, flexWrap: 'wrap'}}>{words.map((w, i) => <div key={w} style={{opacity: interpolate(frame, [24 + i * 10, 36 + i * 10], [0, 1], clamped), transform: `translateY(${interpolate(frame, [24 + i * 10, 36 + i * 10], [14, 0], clamped)}px)`}}><Pill accent={i === 0}>{w}</Pill></div>)}<Pill>CUDA / Vulkan</Pill></div>
      </div>
      <div style={{marginTop: vertical ? 105 : 38, transform: `perspective(1800px) rotateX(${(1 - enter) * 7}deg) translateY(${(1 - enter) * 60}px)`, opacity: enter}}>
        <ScreenshotStage src="ui/chat.png" vertical={vertical}>
          <div style={{position: 'absolute', inset: 0, background: 'rgba(9,15,15,.94)', backdropFilter: 'blur(8px)', padding: vertical ? 42 : 54, color: C.text}}>
            <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: `1px solid ${C.line}`, paddingBottom: 22}}><div style={{font: `700 ${vertical ? 28 : 30}px Segoe UI Variable, sans-serif`}}>本地会话</div><div style={{font: `650 ${vertical ? 25 : 27}px Cascadia Code, monospace`, color: C.cyan}}>● LOCAL ENGINE ONLINE</div></div>
            <div style={{marginTop: 34, marginLeft: '18%', border: `1px solid ${C.line}`, background: C.raised, borderRadius: 18, padding: '22px 26px', font: `600 ${vertical ? 27 : 29}px Segoe UI Variable, Microsoft YaHei, sans-serif`}}>请整理一个清晰、可执行的方案。</div>
            <div style={{marginTop: 26, width: '82%'}}>
              {['分析需求与约束', '组织关键步骤', '生成本地回答'].map((item, i) => {
                const show = interpolate(frame, [34 + i * 24, 48 + i * 24], [0, 1], clamped);
                return <div key={item} style={{display: 'flex', alignItems: 'center', gap: 18, marginTop: 18, opacity: show, transform: `translateY(${(1 - show) * 14}px)`}}><div style={{width: 38, height: 38, borderRadius: 10, border: `1px solid ${C.cyanStrong}`, color: C.cyan, display: 'grid', placeItems: 'center', font: '700 20px Cascadia Code, monospace'}}>{i + 1}</div><div style={{flex: 1, height: 52, borderRadius: 12, background: i === 2 ? 'rgba(60,220,209,.12)' : C.surface, border: `1px solid ${i === 2 ? C.cyanStrong : C.line}`, padding: '11px 18px', font: `600 ${vertical ? 25 : 27}px Segoe UI Variable, Microsoft YaHei, sans-serif`}}>{item}</div></div>;
              })}
            </div>
            <div style={{position: 'absolute', left: 54, right: 54, bottom: 42, display: 'flex', justifyContent: 'space-between', font: `650 ${vertical ? 24 : 26}px Cascadia Code, monospace`, color: C.muted}}><span>PRIVATE · 127.0.0.1</span><span style={{color: C.cyan}}>STREAMING</span></div>
          </div>
          <div style={{position: 'absolute', left: '8%', right: '8%', bottom: '4%', height: 4, background: '#273130'}}><div style={{height: '100%', width: `${stream * 100}%`, background: C.cyan}} /></div>
        </ScreenshotStage>
      </div>
    </AbsoluteFill>
  );
};

const Outro: React.FC<{vertical: boolean; duration: number}> = ({vertical, duration}) => {
  const frame = useCurrentFrame();
  const collapse = ease(interpolate(frame, [0, 70], [0, 1], clamped));
  const lock = spring({frame: frame - 55, fps: 30, config: {damping: 18, stiffness: 120}});
  const features = ['模型发现', '稳定下载', '本地对话', '隐私优先'];
  return (
    <AbsoluteFill style={{opacity: fade(frame, duration), alignItems: 'center', justifyContent: 'center', padding: 60}}>
      <div style={{position: 'absolute', inset: 0}}>
        {features.map((feature, i) => {
          const angle = (Math.PI * 2 * i) / features.length - Math.PI / 2;
          const radius = (vertical ? 340 : 430) * (1 - collapse);
          return <div key={feature} style={{position: 'absolute', left: '50%', top: '46%', transform: `translate(calc(-50% + ${Math.cos(angle) * radius}px), calc(-50% + ${Math.sin(angle) * radius}px)) scale(${1 - collapse * .35})`, opacity: 1 - collapse}}><Pill accent={i === 0}>{feature}</Pill></div>;
        })}
      </div>
      <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', opacity: lock, transform: `scale(${0.86 + lock * 0.14})`}}>
        <Logo size={vertical ? 220 : 175} />
        <div style={{font: `800 ${vertical ? 92 : 96}px Segoe UI Variable, sans-serif`, color: C.text, letterSpacing: 8, marginTop: 30}}>LOCASTRA</div>
        <div style={{font: `650 ${vertical ? 38 : 34}px Segoe UI Variable, Microsoft YaHei, sans-serif`, color: C.muted, marginTop: 12}}>本地模型，从下载到对话。</div>
        <div style={{height: 2, width: vertical ? 680 : 760, background: C.line, margin: '38px 0 26px'}}><div style={{height: '100%', width: `${lock * 100}%`, background: C.cyan}} /></div>
        <Eyebrow vertical={vertical}>0.9.0 · 现在开源 · WINDOWS</Eyebrow>
      </div>
    </AbsoluteFill>
  );
};

const Soundtrack: React.FC<{bgm: boolean}> = ({bgm}) => (
  <>
    {bgm ? <Audio src={staticFile('audio/house-vibez.mp3')} startFrom={7} volume={(f) => Math.min(0.56, f / 50 * 0.56, (900 - f) / 55 * 0.56)} /> : null}
    <Sequence from={96} durationInFrames={45}><Audio src={staticFile('audio/air-zoom-vacuum.mp3')} volume={0.28} /></Sequence>
    <Sequence from={231} durationInFrames={35}><Audio src={staticFile('audio/whoosh-fast.mp3')} volume={0.24} /></Sequence>
    <Sequence from={379} durationInFrames={50}><Audio src={staticFile('audio/sweep-metal-quick.mp3')} volume={0.25} /></Sequence>
    <Sequence from={497} durationInFrames={45}><Audio src={staticFile('audio/air-zoom-vacuum.mp3')} volume={0.24} /></Sequence>
    <Sequence from={734} durationInFrames={55}><Audio src={staticFile('audio/sweep-metal-quick.mp3')} volume={0.26} /></Sequence>
    <Sequence from={776} durationInFrames={40}><Audio src={staticFile('audio/impact-deep-whoosh.mp3')} volume={0.28} /></Sequence>
    <Sequence from={827} durationInFrames={60}><Audio src={staticFile('audio/shimmer-sparkle-sweep.mp3')} volume={0.20} /></Sequence>
  </>
);

export const LocastraPromo: React.FC<{vertical: boolean; bgm: boolean}> = ({vertical, bgm}) => {
  const {width, height} = useVideoConfig();
  return (
    <AbsoluteFill style={{width, height, overflow: 'hidden', color: C.text}}>
      <Grid />
      <Sequence from={0} durationInFrames={104}><Opening vertical={vertical} duration={104} /></Sequence>
      <Sequence from={92} durationInFrames={145}><ProductScene vertical={vertical} duration={145} /></Sequence>
      <Sequence from={225} durationInFrames={160}><ModelFitScene vertical={vertical} duration={160} /></Sequence>
      <Sequence from={373} durationInFrames={130}><DownloadScene vertical={vertical} duration={130} /></Sequence>
      <Sequence from={491} durationInFrames={249}><ChatScene vertical={vertical} duration={249} /></Sequence>
      <Sequence from={728} durationInFrames={172}><Outro vertical={vertical} duration={172} /></Sequence>
      <Soundtrack bgm={bgm} />
    </AbsoluteFill>
  );
};
